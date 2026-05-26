//! Repository contracts for the armory domain.
//!
//! Two repos because they have different access rules:
//! - [`GunRepository`] — per-user. Every query is scoped to a user_id;
//!   no cross-user reads. Lifecycle managed by the user.
//! - [`GunCatalogRepository`] — global, read by all users, written by
//!   admins. Used to autofill firearm specs at the UI level.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::armory::{CatalogEntry, Gun, WeaponClass},
    errors::DomainResult,
};

/// Optional list-filters for guns.
#[derive(Debug, Clone, Default)]
pub struct GunFilter {
    pub class: Option<WeaponClass>,
    pub caliber: Option<String>,
    /// Case-insensitive substring match on manufacturer / model / notes.
    pub q: Option<String>,
}

#[async_trait]
pub trait GunRepository: Send + Sync {
    /// Insert a new gun row. The repo handles serial encryption.
    async fn create(&self, gun: &Gun) -> DomainResult<()>;
    async fn update(&self, gun: &Gun) -> DomainResult<()>;
    async fn delete(&self, user_id: Uuid, id: Uuid) -> DomainResult<()>;

    async fn find(&self, user_id: Uuid, id: Uuid) -> DomainResult<Option<Gun>>;

    async fn list_for_user(
        &self,
        user_id: Uuid,
        filter: &GunFilter,
        limit: i64,
        offset: i64,
        sort: Option<&str>, // already-validated SQL fragment, or None for default
    ) -> DomainResult<(Vec<Gun>, i64)>;
}

#[derive(Debug, Clone, Default)]
pub struct CatalogFilter {
    pub class: Option<WeaponClass>,
    pub caliber: Option<String>,
    /// Substring match across manufacturer + model.
    pub q: Option<String>,
}

#[async_trait]
pub trait GunCatalogRepository: Send + Sync {
    async fn create(&self, entry: &CatalogEntry) -> DomainResult<()>;
    async fn update(&self, entry: &CatalogEntry) -> DomainResult<()>;
    async fn delete(&self, id: Uuid) -> DomainResult<()>;

    async fn find(&self, id: Uuid) -> DomainResult<Option<CatalogEntry>>;

    async fn list(
        &self,
        filter: &CatalogFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<CatalogEntry>, i64)>;
}
