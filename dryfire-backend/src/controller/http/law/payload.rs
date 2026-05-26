use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::law::{
    Law, LawCategory, LawSearchHit, LawTag, LawVersion,
};

// List / filter / search query

#[derive(Debug, Deserialize)]
pub struct LawListQuery {
    pub region: Option<String>,
    pub category_id: Option<Uuid>,
    /// Comma-separated list of tags. OR semantics. e.g. `?any_tags=storage,carry`
    pub any_tags: Option<String>,
    /// Comma-separated list of tags. AND semantics.
    pub all_tags: Option<String>,
    pub updated_after: Option<DateTime<Utc>>,
    pub effective_after: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    /// Free-form search text. Supports `websearch_to_tsquery` syntax:
    /// quoted phrases, `OR`, leading `-` for negation.
    pub q: String,
    pub region: Option<String>,
    pub category_id: Option<Uuid>,
    pub any_tags: Option<String>,
    pub all_tags: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangesSinceQuery {
    /// If omitted, the calling user's `last_visit_at` is used.
    /// If the user has never visited (no recorded last_visit), we fall
    /// back to their account `created_at`.
    pub since: Option<DateTime<Utc>>,
    /// Region. If omitted, the user's profile region is used.
    pub region: Option<String>,
}

/// Parses a comma-separated tag string and silently drops unknown values.
/// (Strict mode would 400; we prefer permissive read semantics.)
pub fn parse_tags_csv(s: Option<&str>) -> Vec<LawTag> {
    let Some(s) = s else { return Vec::new() };
    s.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse::<LawTag>().ok())
        .collect()
}

// Law DTOs

#[derive(Debug, Serialize)]
pub struct LawResponse {
    #[serde(flatten)]
    pub law: Law,
}

#[derive(Debug, Serialize)]
pub struct LawSearchHitResponse {
    #[serde(flatten)]
    pub hit: LawSearchHit,
}

#[derive(Debug, Serialize)]
pub struct LawVersionResponse {
    #[serde(flatten)]
    pub version: LawVersion,
}

#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    #[serde(flatten)]
    pub category: LawCategory,
}

// Admin: create / update law //

#[derive(Debug, Deserialize)]
pub struct CreateLawRequest {
    pub law_key: String,
    pub title: String,
    pub summary: Option<String>,
    pub body: String,
    #[serde(default = "default_region")]
    pub region: String,
    pub category_id: Option<Uuid>,
    #[serde(default)]
    pub tags: Vec<LawTag>,
    pub effective_at: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLawRequest {
    /// Required: identifies which law to update.
    pub law_key: String,
    pub title: String,
    pub summary: Option<String>,
    pub body: String,
    pub region: String,
    pub category_id: Option<Uuid>,
    #[serde(default)]
    pub tags: Vec<LawTag>,
    pub effective_at: NaiveDate,
}

fn default_region() -> String { "RU".into() }

// Admin: categories //

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub code: String,
    pub name: String,
    pub parent_id: Option<Uuid>,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateCategoryRequest {
    #[serde(default)] pub code: Option<String>,
    #[serde(default)] pub name: Option<String>,
    #[serde(default)] pub parent_id: Option<Option<Uuid>>,
    #[serde(default)] pub sort_order: Option<i32>,
}
