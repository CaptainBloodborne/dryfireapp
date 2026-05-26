//! Use cases for the laws domain.
//!
//! Split into:
//! - **Public reads** (any authenticated user): list, search, get,
//!   versions, changes-since-last-visit, category navigation.
//! - **Admin / ingester writes**: CRUD on laws and categories.

use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::law::{Law, LawCategory, LawSearchHit, LawTag, LawVersion},
        errors::{DomainError, DomainResult},
        repositories::law::LawFilter,
        services::audit::AuditEntry,
    },
};

// public

pub struct ListCategoriesUseCase<'a> { pub state: &'a AppState }
impl<'a> ListCategoriesUseCase<'a> {
    pub async fn execute(&self) -> DomainResult<Vec<LawCategory>> {
        self.state.law_repo.category_list().await
    }
}

pub struct ListLawsUseCase<'a> { pub state: &'a AppState }
impl<'a> ListLawsUseCase<'a> {
    pub async fn execute(
        &self, filter: &LawFilter, limit: i64, offset: i64,
    ) -> DomainResult<(Vec<Law>, i64)> {
        self.state.law_repo.list(filter, limit, offset).await
    }
}

pub struct GetLawUseCase<'a> { pub state: &'a AppState }
impl<'a> GetLawUseCase<'a> {
    pub async fn execute(&self, id: Uuid) -> DomainResult<Law> {
        self.state.law_repo.find(id).await?
            .ok_or(DomainError::LawNotFound)
    }
}

pub struct GetLawByKeyUseCase<'a> { pub state: &'a AppState }
impl<'a> GetLawByKeyUseCase<'a> {
    pub async fn execute(&self, law_key: &str) -> DomainResult<Law> {
        self.state.law_repo.find_by_key(law_key).await?
            .ok_or(DomainError::LawNotFound)
    }
}

pub struct SearchLawsUseCase<'a> { pub state: &'a AppState }
impl<'a> SearchLawsUseCase<'a> {
    pub async fn execute(
        &self, query_text: &str, filter: &LawFilter,
        limit: i64, offset: i64,
    ) -> DomainResult<(Vec<LawSearchHit>, i64)> {
        let trimmed = query_text.trim();
        if trimmed.is_empty() {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "search query cannot be empty".into())));
        }
        // Guard against pathological queries — very long strings can
        // make ts_headline slow.
        if trimmed.len() > 512 {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "search query too long (>512 chars)".into())));
        }
        self.state.law_repo.search(trimmed, filter, limit, offset).await
    }
}

pub struct LawVersionsUseCase<'a> { pub state: &'a AppState }
impl<'a> LawVersionsUseCase<'a> {
    pub async fn execute(&self, law_id: Uuid) -> DomainResult<Vec<LawVersion>> {
        // Surface NotFound when the law itself doesn't exist, otherwise
        // a non-existent law would just yield an empty history (which
        // is misleading).
        if self.state.law_repo.find(law_id).await?.is_none() {
            return Err(DomainError::LawNotFound);
        }
        self.state.law_repo.versions(law_id).await
    }
}

/// "What changed since I last visited?" — looks at the calling user's
/// `last_visit_at` (or a fallback supplied by the controller) and
/// returns laws in their region whose `updated_at` is more recent.
pub struct ChangesSinceUseCase<'a> { pub state: &'a AppState }
impl<'a> ChangesSinceUseCase<'a> {
    pub async fn execute(
        &self, region: &str, since: DateTime<Utc>,
        limit: i64, offset: i64,
    ) -> DomainResult<(Vec<Law>, i64)> {
        self.state.law_repo.changes_since(region, since, limit, offset).await
    }
}

// admin

#[derive(Debug)]
pub struct CreateLawInput {
    pub law_key: String,
    pub title: String,
    pub summary: Option<String>,
    pub body: String,
    pub region: String,
    pub category_id: Option<Uuid>,
    pub tags: Vec<LawTag>,
    pub effective_at: chrono::NaiveDate,
}

pub struct CreateLawUseCase<'a> { pub state: &'a AppState }
impl<'a> CreateLawUseCase<'a> {
    #[tracing::instrument(skip(self, input), fields(law_key = %input.law_key))]
    pub async fn execute(&self, admin_id: Uuid, input: CreateLawInput) -> DomainResult<Law> {
        let law = Law::new(
            input.law_key, input.title, input.summary, input.body,
            input.region, input.category_id, input.tags, input.effective_at,
        )?;
        self.state.law_repo.create(&law).await?;
        self.state.audit.record(
            AuditEntry::new("law.create")
                .user(admin_id)
                .resource("law", law.id)
                .metadata(json!({"law_key": law.law_key, "region": law.region})),
        ).await;
        Ok(law)
    }
}

#[derive(Debug)]
pub struct UpdateLawByKeyInput {
    pub law_key: String,
    pub title: String,
    pub summary: Option<String>,
    pub body: String,
    pub region: String,
    pub category_id: Option<Uuid>,
    pub tags: Vec<LawTag>,
    pub effective_at: chrono::NaiveDate,
}

pub struct UpdateLawByKeyUseCase<'a> { pub state: &'a AppState }
impl<'a> UpdateLawByKeyUseCase<'a> {
    pub async fn execute(&self, admin_id: Uuid, input: UpdateLawByKeyInput) -> DomainResult<Law> {
        // Validate via the constructor (then ignore the generated id —
        // the repo updates by `law_key`).
        let candidate = Law::new(
            input.law_key, input.title, input.summary, input.body,
            input.region, input.category_id, input.tags, input.effective_at,
        )?;
        let saved = self.state.law_repo.update_by_key(&candidate).await?;
        self.state.audit.record(
            AuditEntry::new("law.update")
                .user(admin_id)
                .resource("law", saved.id)
                .metadata(json!({
                    "law_key": saved.law_key,
                    "new_version": saved.current_version,
                })),
        ).await;
        Ok(saved)
    }
}

pub struct DeleteLawUseCase<'a> { pub state: &'a AppState }
impl<'a> DeleteLawUseCase<'a> {
    pub async fn execute(&self, admin_id: Uuid, id: Uuid) -> DomainResult<()> {
        self.state.law_repo.delete(id).await?;
        self.state.audit.record(
            AuditEntry::new("law.delete")
                .user(admin_id)
                .resource("law", id),
        ).await;
        Ok(())
    }
}

// ---- categories (admin) ---- //

#[derive(Debug)]
pub struct CreateCategoryInput {
    pub code: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
}

pub struct CreateCategoryUseCase<'a> { pub state: &'a AppState }
impl<'a> CreateCategoryUseCase<'a> {
    pub async fn execute(&self, admin_id: Uuid, input: CreateCategoryInput) -> DomainResult<LawCategory> {
        let c = LawCategory::new(input.code, input.name, input.parent_id, input.sort_order)?;
        self.state.law_repo.category_create(&c).await?;
        self.state.audit.record(
            AuditEntry::new("law_category.create")
                .user(admin_id)
                .resource("law_category", c.id),
        ).await;
        Ok(c)
    }
}

#[derive(Debug, Default)]
pub struct UpdateCategoryInput {
    pub code: Option<String>,
    pub name: Option<String>,
    pub parent_id: Option<Option<Uuid>>,
    pub sort_order: Option<i32>,
}

pub struct UpdateCategoryUseCase<'a> { pub state: &'a AppState }
impl<'a> UpdateCategoryUseCase<'a> {
    pub async fn execute(
        &self, admin_id: Uuid, id: Uuid, input: UpdateCategoryInput,
    ) -> DomainResult<LawCategory> {
        let mut c = self.state.law_repo.category_find(id).await?
            .ok_or(DomainError::LawNotFound)?;
        if let Some(v) = input.code { c.code = v; }
        if let Some(v) = input.name { c.name = v; }
        if let Some(v) = input.parent_id { c.parent_id = v; }
        if let Some(v) = input.sort_order { c.sort_order = v; }
        c.updated_at = Utc::now();
        self.state.law_repo.category_update(&c).await?;
        self.state.audit.record(
            AuditEntry::new("law_category.update")
                .user(admin_id)
                .resource("law_category", id),
        ).await;
        Ok(c)
    }
}

pub struct DeleteCategoryUseCase<'a> { pub state: &'a AppState }
impl<'a> DeleteCategoryUseCase<'a> {
    pub async fn execute(&self, admin_id: Uuid, id: Uuid) -> DomainResult<()> {
        self.state.law_repo.category_delete(id).await?;
        self.state.audit.record(
            AuditEntry::new("law_category.delete")
                .user(admin_id)
                .resource("law_category", id),
        ).await;
        Ok(())
    }
}
