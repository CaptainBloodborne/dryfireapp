// src/application/law_use_cases.rs

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::law::{Law, LawTag},
        errors::{DomainError, DomainResult},
        repositories::armory::PageQuery,
    },
};

pub struct ListLawsUseCase<'a> { pub state: &'a AppState }
impl<'a> ListLawsUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(
        &self, region: Option<&str>, tags: &[LawTag], page: PageQuery,
    ) -> DomainResult<(Vec<Law>, i64)> {
        self.state.law_repo.list(region, tags, page).await
    }
}

pub struct SearchLawsUseCase<'a> { pub state: &'a AppState }
impl<'a> SearchLawsUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(
        &self, region: Option<&str>, q: &str, page: PageQuery,
    ) -> DomainResult<(Vec<Law>, i64)> {
        self.state.law_repo.full_text(region, q, page).await
    }
}

pub struct LawUpdatesSinceUseCase<'a> { pub state: &'a AppState }
impl<'a> LawUpdatesSinceUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(
        &self, region: Option<&str>, since: DateTime<Utc>,
    ) -> DomainResult<Vec<Law>> {
        self.state.law_repo.updates_since(region, since).await
    }
}

/// Admin only.
pub struct UpsertLawUseCase<'a> { pub state: &'a AppState }
impl<'a> UpsertLawUseCase<'a> {
    pub fn new(state: &'a AppState) -> Self { Self { state } }
    pub async fn execute(&self, law: Law) -> DomainResult<()> {
        self.state.law_repo.upsert(&law).await
    }
}