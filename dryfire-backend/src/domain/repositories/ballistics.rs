use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{
    entities::ballistics::BallisticProfile,
    errors::DomainResult,
};

#[async_trait]
pub trait BallisticProfileRepository: Send + Sync {
    async fn create(&self, p: &BallisticProfile) -> DomainResult<()>;
    async fn update(&self, p: &BallisticProfile) -> DomainResult<()>;
    async fn find(&self, user_id: Uuid, id: Uuid) -> DomainResult<Option<BallisticProfile>>;
    async fn list_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<BallisticProfile>, i64)>;
    async fn delete(&self, user_id: Uuid, id: Uuid) -> DomainResult<()>;
}
