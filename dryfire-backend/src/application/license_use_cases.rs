//! License use cases.

use chrono::{NaiveDate, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::license::{License, LicenseType},
        errors::{DomainError, DomainResult},
        repositories::license::{LicenseDeadline, LicenseFilter},
        services::audit::AuditEntry,
    },
};

// User-facing

#[derive(Debug)]
pub struct RegisterLicenseInput {
    pub user_id: Uuid,
    pub license_type_id: Option<Uuid>,
    pub license_number: String,
    pub issuer: String,
    pub issued_at: NaiveDate,
    /// If omitted AND `license_type_id` resolves to a type with a
    /// `validity_days`, expiry is auto-computed.
    pub expires_at: Option<NaiveDate>,
    pub notes: Option<String>,
    pub scan_url: Option<String>,
    pub gun_ids: Vec<Uuid>,
}

pub struct RegisterLicenseUseCase<'a> { pub state: &'a AppState }
impl<'a> RegisterLicenseUseCase<'a> {
    #[tracing::instrument(skip(self, input), fields(user_id = %input.user_id))]
    pub async fn execute(&self, input: RegisterLicenseInput) -> DomainResult<License> {
        // Resolve expiry: explicit > type-derived > error.
        let expires_at = match input.expires_at {
            Some(d) => d,
            None => {
                let Some(type_id) = input.license_type_id else {
                    return Err(DomainError::Validation(
                        crate::domain::errors::ValidationError::Custom(
                            "expires_at is required when license_type_id is not set".into())));
                };
                let t = self.state.license_repo.type_find(type_id).await?
                    .ok_or(DomainError::LicenseNotFound)?;
                let Some(days) = t.validity_days else {
                    return Err(DomainError::Validation(
                        crate::domain::errors::ValidationError::Custom(
                            format!("license type `{}` has no default validity; \
                                     expires_at must be supplied explicitly", t.code))));
                };
                License::compute_expiry(input.issued_at, days)
            }
        };

        let license = License::register(
            input.user_id,
            input.license_type_id,
            input.license_number,
            input.issuer,
            input.issued_at,
            expires_at,
            input.notes,
            input.scan_url,
            input.gun_ids,
        )?;
        self.state.license_repo.create(&license).await?;

        self.state.audit.record(
            AuditEntry::new("license.create")
                .user(input.user_id)
                .resource("license", license.id())
                .metadata(json!({
                    "number": license.license_number(),
                    "issuer": license.issuer(),
                    "expires_at": license.expires_at(),
                })),
        ).await;
        Ok(license)
    }
}

pub struct ListLicensesUseCase<'a> { pub state: &'a AppState }
impl<'a> ListLicensesUseCase<'a> {
    pub async fn execute(
        &self, user_id: Uuid, filter: &LicenseFilter,
        today: NaiveDate, limit: i64, offset: i64,
    ) -> DomainResult<(Vec<License>, i64)> {
        self.state.license_repo.list_for_user(user_id, filter, today, limit, offset).await
    }
}

pub struct GetLicenseUseCase<'a> { pub state: &'a AppState }
impl<'a> GetLicenseUseCase<'a> {
    pub async fn execute(&self, user_id: Uuid, id: Uuid) -> DomainResult<License> {
        self.state.license_repo.find(user_id, id).await?
            .ok_or(DomainError::LicenseExpired)
    }
}

#[derive(Debug, Default)]
pub struct UpdateLicenseInput {
    pub license_type_id: Option<Option<Uuid>>,
    pub license_number: Option<String>,
    pub issuer: Option<String>,
    pub issued_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub notes: Option<Option<String>>,
    pub scan_url: Option<Option<String>>,
    pub gun_ids: Option<Vec<Uuid>>,
}

pub struct UpdateLicenseUseCase<'a> { pub state: &'a AppState }
impl<'a> UpdateLicenseUseCase<'a> {
    pub async fn execute(
        &self, user_id: Uuid, id: Uuid, input: UpdateLicenseInput,
    ) -> DomainResult<License> {
        let mut l = self.state.license_repo.find(user_id, id).await?
            .ok_or(DomainError::LicenseNotFound)?;
        l.apply_update(
            input.license_type_id, input.license_number, input.issuer,
            input.issued_at, input.expires_at,
            input.notes, input.scan_url, input.gun_ids,
        )?;
        self.state.license_repo.update(&l).await?;
        self.state.audit.record(
            AuditEntry::new("license.update")
                .user(user_id)
                .resource("license", id),
        ).await;
        Ok(l)
    }
}

pub struct DeleteLicenseUseCase<'a> { pub state: &'a AppState }
impl<'a> DeleteLicenseUseCase<'a> {
    pub async fn execute(&self, user_id: Uuid, id: Uuid) -> DomainResult<()> {
        self.state.license_repo.delete(user_id, id).await?;
        self.state.audit.record(
            AuditEntry::new("license.delete")
                .user(user_id)
                .resource("license", id),
        ).await;
        Ok(())
    }
}

pub struct DeadlinesInRangeUseCase<'a> { pub state: &'a AppState }
impl<'a> DeadlinesInRangeUseCase<'a> {
    pub async fn execute(
        &self, user_id: Uuid, from: NaiveDate, to: NaiveDate,
    ) -> DomainResult<Vec<LicenseDeadline>> {
        if to < from {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "`to` must be on or after `from`".into())));
        }
        self.state.license_repo.deadlines_in_range(user_id, from, to).await
    }
}

// ====================== Admin: license types ====================== //

#[derive(Debug)]
pub struct CreateLicenseTypeInput {
    pub code: String,
    pub name: String,
    pub region: String,
    pub validity_days: Option<i32>,
    pub instructions: Option<String>,
}

pub struct CreateLicenseTypeUseCase<'a> { pub state: &'a AppState }
impl<'a> CreateLicenseTypeUseCase<'a> {
    pub async fn execute(&self, admin_id: Uuid, input: CreateLicenseTypeInput)
        -> DomainResult<LicenseType>
    {
        let t = LicenseType::new(
            input.code, input.name, input.region,
            input.validity_days, input.instructions,
        )?;
        self.state.license_repo.type_create(&t).await?;
        self.state.audit.record(
            AuditEntry::new("license_type.create")
                .user(admin_id)
                .resource("license_type", t.id),
        ).await;
        Ok(t)
    }
}

#[derive(Debug, Default)]
pub struct UpdateLicenseTypeInput {
    pub code: Option<String>,
    pub name: Option<String>,
    pub region: Option<String>,
    pub validity_days: Option<Option<i32>>,
    pub instructions: Option<Option<String>>,
}

pub struct UpdateLicenseTypeUseCase<'a> { pub state: &'a AppState }
impl<'a> UpdateLicenseTypeUseCase<'a> {
    pub async fn execute(
        &self, admin_id: Uuid, id: Uuid, input: UpdateLicenseTypeInput,
    ) -> DomainResult<LicenseType> {
        let mut t = self.state.license_repo.type_find(id).await?
            .ok_or(DomainError::LicenseNotFound)?;
        if let Some(c) = input.code { t.code = c; }
        if let Some(n) = input.name { t.name = n; }
        if let Some(r) = input.region { t.region = r; }
        if let Some(d) = input.validity_days { t.validity_days = d; }
        if let Some(i) = input.instructions { t.instructions = i; }
        t.updated_at = Utc::now();
        self.state.license_repo.type_update(&t).await?;
        self.state.audit.record(
            AuditEntry::new("license_type.update")
                .user(admin_id)
                .resource("license_type", id),
        ).await;
        Ok(t)
    }
}

pub struct DeleteLicenseTypeUseCase<'a> { pub state: &'a AppState }
impl<'a> DeleteLicenseTypeUseCase<'a> {
    pub async fn execute(&self, admin_id: Uuid, id: Uuid) -> DomainResult<()> {
        self.state.license_repo.type_delete(id).await?;
        self.state.audit.record(
            AuditEntry::new("license_type.delete")
                .user(admin_id)
                .resource("license_type", id),
        ).await;
        Ok(())
    }
}

pub struct ListLicenseTypesUseCase<'a> { pub state: &'a AppState }
impl<'a> ListLicenseTypesUseCase<'a> {
    pub async fn execute(
        &self, region: Option<&str>, limit: i64, offset: i64,
    ) -> DomainResult<(Vec<LicenseType>, i64)> {
        self.state.license_repo.type_list(region, limit, offset).await
    }
}

pub struct GetLicenseTypeUseCase<'a> { pub state: &'a AppState }
impl<'a> GetLicenseTypeUseCase<'a> {
    pub async fn execute(&self, id: Uuid) -> DomainResult<LicenseType> {
        self.state.license_repo.type_find(id).await?
            .ok_or(DomainError::LicenseNotFound)
    }
}
