// src/controller/http/law/payload.rs

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::law::{Law, LawTag};

#[derive(Debug, Deserialize)]
pub struct ListLawsQuery {
    pub region: Option<String>,
    /// Comma-separated list of tags, e.g. `?tags=storage,hunting`.
    pub tags: Option<String>,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_size")]
    pub size: u32,
}
fn default_page() -> u32 { 1 }
fn default_size() -> u32 { 20 }

impl ListLawsQuery {
    pub fn parse_tags(&self) -> Result<Vec<LawTag>, crate::domain::errors::ValidationError> {
        match &self.tags {
            None => Ok(vec![]),
            Some(s) => s.split(',')
                .filter(|t| !t.trim().is_empty())
                .map(|t| t.trim().parse::<LawTag>())
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchLawsQuery {
    pub q: String,
    pub region: Option<String>,
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_size")]
    pub size: u32,
}

#[derive(Debug, Deserialize)]
pub struct UpdatesSinceQuery {
    pub since: DateTime<Utc>,
    pub region: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LawSummary {
    pub id: Uuid,
    pub region: String,
    pub slug: String,
    pub title: String,
    pub version: i32,
    pub tags: Vec<String>,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Law> for LawSummary {
    fn from(l: &Law) -> Self {
        Self {
            id: l.id,
            region: l.region.clone(),
            slug: l.slug.clone(),
            title: l.title.clone(),
            version: l.version,
            tags: l.tags.iter().map(|t| t.as_str().into()).collect(),
            published_at: l.published_at,
            updated_at: l.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LawFull {
    pub id: Uuid,
    pub region: String,
    pub slug: String,
    pub title: String,
    pub body: String,
    pub version: i32,
    pub tags: Vec<String>,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
impl From<&Law> for LawFull {
    fn from(l: &Law) -> Self {
        Self {
            id: l.id,
            region: l.region.clone(),
            slug: l.slug.clone(),
            title: l.title.clone(),
            body: l.body.clone(),
            version: l.version,
            tags: l.tags.iter().map(|t| t.as_str().into()).collect(),
            published_at: l.published_at,
            updated_at: l.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpsertLawRequest {
    pub region: String,
    pub slug: String,
    pub title: String,
    pub body: String,
    pub version: i32,
    #[serde(default)]
    pub tags: Vec<String>,
    pub published_at: Option<DateTime<Utc>>,
}