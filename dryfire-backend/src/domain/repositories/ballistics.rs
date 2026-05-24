// src/domain/repositories/ballistics.rs

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::ballistics::BallisticProfile,
    errors::DomainResult,
    repositories::armory::PageQuery,
};

#[async_trait]
pub trait BallisticProfileRepository: Send + Sync {
    async fn create(&self, profile: &BallisticProfile) -> DomainResult<()>;
    async fn find_by_id(&self, id: Uuid, owner_id: Uuid) -> DomainResult<Option<BallisticProfile>>;
    async fn list(&self, owner_id: Uuid, page: PageQuery)
        -> DomainResult<(Vec<BallisticProfile>, i64)>;
    async fn update(&self, profile: &BallisticProfile) -> DomainResult<()>;
    async fn delete(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()>;
}