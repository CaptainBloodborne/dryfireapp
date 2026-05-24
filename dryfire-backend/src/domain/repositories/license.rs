// src/domain/repositories/license.rs

use async_trait::async_trait;
use chrono::NaiveDate;
use uuid::Uuid;

use crate::domain::{
    entities::license::License,
    errors::DomainResult,
    repositories::armory::PageQuery,
};

#[async_trait]
pub trait LicenseRepository: Send + Sync {
    async fn create(&self, lic: &License) -> DomainResult<()>;
    async fn find_by_id(&self, id: Uuid, owner_id: Uuid) -> DomainResult<Option<License>>;
    async fn list(&self, owner_id: Uuid, page: PageQuery) -> DomainResult<(Vec<License>, i64)>;
    async fn update(&self, lic: &License) -> DomainResult<()>;
    async fn delete(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()>;

    async fn deadlines_in(&self, owner_id: Uuid, from: NaiveDate, to: NaiveDate)
        -> DomainResult<Vec<License>>;

    async fn expiring_in_days_globally(&self, days_before: i32)
        -> DomainResult<Vec<License>>;

    async fn mark_notified(&self, license_id: Uuid, days_before: i32)
        -> DomainResult<bool>;

    async fn link_gun(&self, license_id: Uuid, gun_id: Uuid) -> DomainResult<()>;
    async fn unlink_gun(&self, license_id: Uuid, gun_id: Uuid) -> DomainResult<()>;
    async fn list_gun_links(&self, license_id: Uuid) -> DomainResult<Vec<Uuid>>;
}