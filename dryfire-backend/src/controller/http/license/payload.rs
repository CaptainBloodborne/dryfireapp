use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::license::{License, LicenseType};

// License DTOs

#[derive(Debug, Deserialize)]
pub struct CreateLicenseRequest {
    pub license_type_id: Option<Uuid>,
    pub license_number: String,
    pub issuer: String,
    pub issued_at: NaiveDate,
    /// May be omitted when `license_type_id` resolves to a type that
    /// has a `validity_days`.
    pub expires_at: Option<NaiveDate>,
    pub notes: Option<String>,
    pub scan_url: Option<String>,
    #[serde(default)]
    pub gun_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateLicenseRequest {
    #[serde(default)] pub license_type_id: Option<Option<Uuid>>,
    #[serde(default)] pub license_number: Option<String>,
    #[serde(default)] pub issuer: Option<String>,
    #[serde(default)] pub issued_at: Option<NaiveDate>,
    #[serde(default)] pub expires_at: Option<NaiveDate>,
    #[serde(default)] pub notes: Option<Option<String>>,
    #[serde(default)] pub scan_url: Option<Option<String>>,
    /// Full replacement of the linked-guns set. Send `[]` to detach all.
    #[serde(default)] pub gun_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Serialize)]
pub struct LicenseResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub license_type_id: Option<Uuid>,
    pub license_number: String,
    pub issuer: String,
    pub issued_at: NaiveDate,
    pub expires_at: NaiveDate,
    pub is_expired: bool,
    pub days_until_expiry: i64,
    pub notes: Option<String>,
    pub scan_url: Option<String>,
    pub gun_ids: Vec<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LicenseResponse {
    pub fn from_license(l: &License, today: NaiveDate) -> Self {
        Self {
            id: l.id(),
            user_id: l.user_id(),
            license_type_id: l.license_type_id(),
            license_number: l.license_number().to_string(),
            issuer: l.issuer().to_string(),
            issued_at: l.issued_at(),
            expires_at: l.expires_at(),
            is_expired: l.is_expired(today),
            days_until_expiry: l.days_until_expiry(today),
            notes: l.notes().map(str::to_string),
            scan_url: l.scan_url().map(str::to_string),
            gun_ids: l.gun_ids().to_vec(),
            created_at: l.created_at(),
            updated_at: l.updated_at(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LicenseListQuery {
    pub gun_id: Option<Uuid>,
    pub type_id: Option<Uuid>,
    /// `expired=true` - only expired, `false` - only valid.
    pub expired: Option<bool>,
    pub q: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DeadlinesQuery {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct DeadlineResponse {
    pub license_id: Uuid,
    pub license_number: String,
    pub issuer: String,
    pub expires_at: NaiveDate,
}

// LicenseType DTOs

#[derive(Debug, Deserialize)]
pub struct CreateLicenseTypeRequest {
    pub code: String,
    pub name: String,
    #[serde(default = "default_region")]
    pub region: String,
    pub validity_days: Option<i32>,
    pub instructions: Option<String>,
}
fn default_region() -> String { "RU".into() }

#[derive(Debug, Deserialize, Default)]
pub struct UpdateLicenseTypeRequest {
    #[serde(default)] pub code: Option<String>,
    #[serde(default)] pub name: Option<String>,
    #[serde(default)] pub region: Option<String>,
    #[serde(default)] pub validity_days: Option<Option<i32>>,
    #[serde(default)] pub instructions: Option<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct LicenseTypeResponse {
    #[serde(flatten)]
    pub entry: LicenseType,
}

#[derive(Debug, Deserialize)]
pub struct LicenseTypeListQuery {
    pub region: Option<String>,
}
