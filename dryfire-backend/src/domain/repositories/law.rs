//! Repository contracts for the laws domain.
//!
//! Two roles use this repo:
//! - Anonymous reads (any authenticated user): list, get, search,
//!   "what changed since". Region-scoped from the user's profile.
//! - Admin writes: CRUD on laws and categories. Ingesters (background
//!   jobs that scrape source feeds) also call these mutators.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::law::{Law, LawCategory, LawSearchHit, LawTag, LawVersion},
    errors::DomainResult,
};

#[derive(Debug, Clone, Default)]
pub struct LawFilter {
    pub region: Option<String>,
    pub category_id: Option<Uuid>,
    pub any_tags: Vec<LawTag>,
    pub all_tags: Vec<LawTag>,
    pub updated_after: Option<DateTime<Utc>>,
    pub effective_after: Option<NaiveDate>,
}

#[async_trait]
pub trait LawRepository: Send + Sync {
    // categories 
    async fn category_create(&self, c: &LawCategory) -> DomainResult<()>;
    async fn category_update(&self, c: &LawCategory) -> DomainResult<()>;
    async fn category_delete(&self, id: Uuid) -> DomainResult<()>;
    async fn category_find(&self, id: Uuid) -> DomainResult<Option<LawCategory>>;
    async fn category_list(&self) -> DomainResult<Vec<LawCategory>>;

    // laws (current versions) 
    async fn create(&self, law: &Law) -> DomainResult<()>;
    async fn update_by_key(&self, law: &Law) -> DomainResult<Law>;
    async fn delete(&self, id: Uuid) -> DomainResult<()>;

    async fn find(&self, id: Uuid) -> DomainResult<Option<Law>>;
    async fn find_by_key(&self, law_key: &str) -> DomainResult<Option<Law>>;

    async fn list(
        &self,
        filter: &LawFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<Law>, i64)>;

    // search 
    /// Full-text search. `query_text` is a free-form user query; the
    /// repo converts it to a `tsquery` with `websearch_to_tsquery`
    async fn search(
        &self,
        query_text: &str,
        filter: &LawFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<LawSearchHit>, i64)>;

    // history
    async fn versions(&self, law_id: Uuid) -> DomainResult<Vec<LawVersion>>;

    /// "What changed since the user last visited" — every law whose
    /// `updated_at > since`, scoped by the user's region.
    async fn changes_since(
        &self,
        region: &str,
        since: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<Law>, i64)>;
}
