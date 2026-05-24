// src/domain/repositories/law.rs

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::law::{Law, LawTag},
    errors::DomainResult,
    repositories::armory::PageQuery,
};

#[async_trait]
pub trait LawRepository: Send + Sync {
    async fn upsert(&self, law: &Law) -> DomainResult<()>;
    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<Law>>;
    async fn list(
        &self,
        region: Option<&str>,
        tags: &[LawTag],
        page: PageQuery,
    ) -> DomainResult<(Vec<Law>, i64)>;
    async fn full_text(
        &self,
        region: Option<&str>,
        query: &str,
        page: PageQuery,
    ) -> DomainResult<(Vec<Law>, i64)>;
    async fn updates_since(
        &self,
        region: Option<&str>,
        since: DateTime<Utc>,
    ) -> DomainResult<Vec<Law>>;
    async fn delete(&self, id: Uuid) -> DomainResult<()>;
}