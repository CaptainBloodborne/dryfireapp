use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::license::License;

#[derive(Debug, Deserialize)]
pub struct CreateLicenseRequest {
    pub kind: String,
    pub issuing_org: String,
    pub issued_at: NaiveDate,
    /// If omitted, the use case derives a default validity from `kind`.
    pub expires_at: Option<NaiveDate>,
    pub document_url: Option<String>,
    pub instructions: Option<String>,
    #[serde(default)]
    pub linked_gun_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLicenseRequest {
    pub issuing_org: Option<String>,
    pub issued_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub document_url: Option<String>,
    pub instructions: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LicenseResponse {
    pub id: Uuid,
    pub kind: String,
    pub issuing_org: String,
    pub issued_at: NaiveDate,
    pub expires_at: NaiveDate,
    pub status: String,
    pub document_url: Option<String>,
    pub instructions: Option<String>,
    pub days_until_expiry: i64,
    pub is_expired: bool,
    pub created_at: DateTime<Utc>,
}

impl From<&License> for LicenseResponse {
    fn from(l: &License) -> Self {
        let today = chrono::Utc::now().date_naive();
        Self {
            id: l.id(),
            kind: l.kind().as_str().into(),
            issuing_org: l.issuing_org().into(),
            issued_at: l.issued_at(),
            expires_at: l.expires_at(),
            status: l.status().as_str().into(),
            document_url: l.document_url().map(str::to_string),
            instructions: l.instructions().map(str::to_string),
            days_until_expiry: l.days_until_expiry(today),
            is_expired: l.is_expired(today),
            created_at: l.created_at(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DeadlinesQuery {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct LinkGunRequest {
    pub gun_id: Uuid,
}