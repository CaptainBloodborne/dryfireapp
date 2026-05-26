use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::scope::ScopeProfile,
    errors::DomainResult,
};

#[async_trait]
pub trait ScopeProfileRepository: Send + Sync {
    async fn create(&self, p: &ScopeProfile) -> DomainResult<()>;
    async fn update(&self, p: &ScopeProfile) -> DomainResult<()>;
    async fn find(&self, user_id: Uuid, id: Uuid) -> DomainResult<Option<ScopeProfile>>;
    async fn list_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<ScopeProfile>, i64)>;
    async fn delete(&self, user_id: Uuid, id: Uuid) -> DomainResult<()>;
}
