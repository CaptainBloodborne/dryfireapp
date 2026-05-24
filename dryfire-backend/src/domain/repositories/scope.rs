use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::scope::ScopeProfile,
    errors::DomainResult,
    repositories::armory::PageQuery,
};

#[async_trait]
pub trait ScopeProfileRepository: Send + Sync {
    async fn create(&self, profile: &ScopeProfile) -> DomainResult<()>;
    async fn find_by_id(&self, id: Uuid, owner_id: Uuid)
        -> DomainResult<Option<ScopeProfile>>;
    async fn list(&self, owner_id: Uuid, page: PageQuery)
        -> DomainResult<(Vec<ScopeProfile>, i64)>;
    async fn update(&self, profile: &ScopeProfile) -> DomainResult<()>;
    async fn delete(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()>;
}