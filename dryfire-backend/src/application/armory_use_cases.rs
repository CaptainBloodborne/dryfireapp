//! Armory use cases.
//!
//! Two groups:
//!
//! - **User-facing** gun CRUD — operates on the calling user's records
//!   only. Every method takes `user_id` and the repo's WHERE clause
//!   enforces ownership.
//! - **Catalog** — read access for everyone, write access for admins
//!   only (enforced at the HTTP layer with `require_admin`).

use chrono::NaiveDate;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::armory::{CatalogEntry, Gun, WeaponClass},
        errors::{DomainError, DomainResult},
        repositories::armory::{CatalogFilter, GunFilter},
        services::audit::AuditEntry,
    },
};

// Register gun

#[derive(Debug)]
pub struct RegisterGunInput {
    pub user_id: Uuid,
    pub catalog_id: Option<Uuid>,
    pub manufacturer: String,
    pub model: String,
    pub class: WeaponClass,
    pub caliber: String,
    pub serial: SecretString,
    pub date_of_purchase: NaiveDate,
    pub photo_url: Option<String>,
    pub notes: Option<String>,
}

pub struct RegisterGunUseCase<'a> { pub state: &'a AppState }
impl<'a> RegisterGunUseCase<'a> {
    #[tracing::instrument(skip(self, input), fields(user_id = %input.user_id))]
    pub async fn execute(&self, input: RegisterGunInput) -> DomainResult<Gun> {
        let gun = Gun::register(
            input.user_id, input.catalog_id,
            input.manufacturer, input.model, input.class, input.caliber,
            input.serial, input.date_of_purchase,
            input.photo_url, input.notes,
        )?;
        self.state.gun_repo.create(&gun).await?;

        self.state.audit.record(
            AuditEntry::new("gun.create")
                .user(input.user_id)
                .resource("gun", gun.id())
                .metadata(json!({
                    "manufacturer": gun.manufacturer(),
                    "model": gun.model(),
                    "class": gun.class().as_str(),
                    "caliber": gun.caliber(),
                })),
        ).await;
        Ok(gun)
    }
}

// List / get

pub struct ListGunsUseCase<'a> { pub state: &'a AppState }
impl<'a> ListGunsUseCase<'a> {
    pub async fn execute(
        &self,
        user_id: Uuid,
        filter: &GunFilter,
        limit: i64, offset: i64,
        sort: Option<&str>,
    ) -> DomainResult<(Vec<Gun>, i64)> {
        self.state.gun_repo.list_for_user(user_id, filter, limit, offset, sort).await
    }
}

pub struct GetGunUseCase<'a> { pub state: &'a AppState }
impl<'a> GetGunUseCase<'a> {
    pub async fn execute(&self, user_id: Uuid, id: Uuid) -> DomainResult<Gun> {
        self.state.gun_repo.find(user_id, id).await?
            .ok_or(DomainError::GunNotFound)
    }
}

// Update

#[derive(Debug, Default)]
pub struct UpdateGunInput {
    pub catalog_id: Option<Option<Uuid>>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub class: Option<WeaponClass>,
    pub caliber: Option<String>,
    pub serial: Option<SecretString>,   // explicit; we never re-encrypt
                                         // the same value unnecessarily
    pub date_of_purchase: Option<NaiveDate>,
    pub photo_url: Option<Option<String>>,
    pub notes: Option<Option<String>>,
}

pub struct UpdateGunUseCase<'a> { pub state: &'a AppState }
impl<'a> UpdateGunUseCase<'a> {
    #[tracing::instrument(skip(self, input), fields(user_id = %user_id, gun_id = %id))]
    pub async fn execute(
        &self, user_id: Uuid, id: Uuid, input: UpdateGunInput,
    ) -> DomainResult<Gun> {
        let mut gun = self.state.gun_repo.find(user_id, id).await?
            .ok_or(DomainError::GunNotFound)?;

        gun.apply_update(
            input.catalog_id, input.manufacturer, input.model,
            input.class, input.caliber, input.date_of_purchase,
            input.photo_url, input.notes,
        )?;

        if let Some(new_serial) = input.serial {

            let new_serial_str = new_serial.expose_secret().to_string();
            // Simple validation: non-empty, length cap.
            if new_serial_str.trim().is_empty() {
                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        "serial cannot be empty".into())));
            }
            if new_serial_str.len() > 64 {
                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        "serial too long".into())));
            }
            // Rehydrate with the new secret.
            gun = Gun::rehydrate(
                gun.id(), gun.user_id(), gun.catalog_id(),
                gun.manufacturer().to_string(), gun.model().to_string(),
                gun.class(), gun.caliber().to_string(),
                SecretString::from(new_serial_str),
                gun.date_of_purchase(),
                gun.photo_url().map(str::to_string),
                gun.notes().map(str::to_string),
                gun.created_at(), gun.updated_at(),
            );
        }

        self.state.gun_repo.update(&gun).await?;

        self.state.audit.record(
            AuditEntry::new("gun.update")
                .user(user_id)
                .resource("gun", id),
        ).await;
        Ok(gun)
    }
}

// Delete

pub struct DeleteGunUseCase<'a> { pub state: &'a AppState }
impl<'a> DeleteGunUseCase<'a> {
    #[tracing::instrument(skip(self), fields(user_id = %user_id, gun_id = %id))]
    pub async fn execute(&self, user_id: Uuid, id: Uuid) -> DomainResult<()> {
        self.state.gun_repo.delete(user_id, id).await?;
        self.state.audit.record(
            AuditEntry::new("gun.delete")
                .user(user_id)
                .resource("gun", id),
        ).await;
        Ok(())
    }
}

// Catalog

pub struct ListCatalogUseCase<'a> { pub state: &'a AppState }
impl<'a> ListCatalogUseCase<'a> {
    pub async fn execute(
        &self,
        filter: &CatalogFilter,
        limit: i64, offset: i64,
    ) -> DomainResult<(Vec<CatalogEntry>, i64)> {
        self.state.gun_catalog_repo.list(filter, limit, offset).await
    }
}

pub struct GetCatalogEntryUseCase<'a> { pub state: &'a AppState }
impl<'a> GetCatalogEntryUseCase<'a> {
    pub async fn execute(&self, id: Uuid) -> DomainResult<CatalogEntry> {
        self.state.gun_catalog_repo.find(id).await?
            .ok_or(DomainError::GunNotFound)
    }
}

// admin

#[derive(Debug)]
pub struct CreateCatalogInput {
    pub manufacturer: String,
    pub model: String,
    pub class: WeaponClass,
    pub caliber: String,
    pub barrel_length_mm: Option<i32>,
    pub weight_g: Option<i32>,
    pub capacity: Option<i32>,
    pub notes: Option<String>,
}

pub struct CreateCatalogEntryUseCase<'a> { pub state: &'a AppState }
impl<'a> CreateCatalogEntryUseCase<'a> {
    #[tracing::instrument(skip(self, input))]
    pub async fn execute(&self, admin_id: Uuid, input: CreateCatalogInput) -> DomainResult<CatalogEntry> {
        let entry = CatalogEntry::new(
            input.manufacturer, input.model, input.class, input.caliber,
            input.barrel_length_mm, input.weight_g, input.capacity, input.notes,
        )?;
        self.state.gun_catalog_repo.create(&entry).await?;
        self.state.audit.record(
            AuditEntry::new("catalog.create")
                .user(admin_id)
                .resource("catalog_entry", entry.id),
        ).await;
        Ok(entry)
    }
}

#[derive(Debug, Default)]
pub struct UpdateCatalogInput {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub class: Option<WeaponClass>,
    pub caliber: Option<String>,
    pub barrel_length_mm: Option<Option<i32>>,
    pub weight_g: Option<Option<i32>>,
    pub capacity: Option<Option<i32>>,
    pub notes: Option<Option<String>>,
}

pub struct UpdateCatalogEntryUseCase<'a> { pub state: &'a AppState }
impl<'a> UpdateCatalogEntryUseCase<'a> {
    pub async fn execute(
        &self, admin_id: Uuid, id: Uuid, input: UpdateCatalogInput,
    ) -> DomainResult<CatalogEntry> {
        let mut e = self.state.gun_catalog_repo.find(id).await?
            .ok_or(DomainError::GunNotFound)?;
        if let Some(v) = input.manufacturer { e.manufacturer = v; }
        if let Some(v) = input.model { e.model = v; }
        if let Some(v) = input.class { e.class = v; }
        if let Some(v) = input.caliber { e.caliber = v; }
        if let Some(v) = input.barrel_length_mm { e.barrel_length_mm = v; }
        if let Some(v) = input.weight_g { e.weight_g = v; }
        if let Some(v) = input.capacity { e.capacity = v; }
        if let Some(v) = input.notes { e.notes = v; }
        e.updated_at = chrono::Utc::now();
        self.state.gun_catalog_repo.update(&e).await?;
        self.state.audit.record(
            AuditEntry::new("catalog.update")
                .user(admin_id)
                .resource("catalog_entry", id),
        ).await;
        Ok(e)
    }
}

pub struct DeleteCatalogEntryUseCase<'a> { pub state: &'a AppState }
impl<'a> DeleteCatalogEntryUseCase<'a> {
    pub async fn execute(&self, admin_id: Uuid, id: Uuid) -> DomainResult<()> {
        self.state.gun_catalog_repo.delete(id).await?;
        self.state.audit.record(
            AuditEntry::new("catalog.delete")
                .user(admin_id)
                .resource("catalog_entry", id),
        ).await;
        Ok(())
    }
}
