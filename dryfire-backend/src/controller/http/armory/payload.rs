use chrono::{DateTime, NaiveDate, Utc};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::armory::{CatalogEntry, Gun, WeaponClass};

// ---------------- Gun DTOs ---------------- //

#[derive(Debug, Deserialize)]
pub struct CreateGunRequest {
    pub catalog_id: Option<Uuid>,
    pub manufacturer: String,
    pub model: String,
    pub class: WeaponClass,
    pub caliber: String,
    pub serial: SecretString,
    pub date_of_purchase: NaiveDate,
    pub photo_url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateGunRequest {
    /// `Some(None)` clears; `Some(Some(...))` sets; missing leaves alone.
    #[serde(default)] pub catalog_id: Option<Option<Uuid>>,
    #[serde(default)] pub manufacturer: Option<String>,
    #[serde(default)] pub model: Option<String>,
    #[serde(default)] pub class: Option<WeaponClass>,
    #[serde(default)] pub caliber: Option<String>,
    #[serde(default)] pub serial: Option<SecretString>,
    #[serde(default)] pub date_of_purchase: Option<NaiveDate>,
    #[serde(default)] pub photo_url: Option<Option<String>>,
    #[serde(default)] pub notes: Option<Option<String>>,
}

/// Default Gun response. The serial number is **never** included; only
/// the last-4 hint is. Use the dedicated /serial endpoint to reveal it.
#[derive(Debug, Serialize)]
pub struct GunResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub catalog_id: Option<Uuid>,
    pub manufacturer: String,
    pub model: String,
    pub class: String,
    pub caliber: String,
    pub serial_last4: String,
    pub date_of_purchase: NaiveDate,
    pub photo_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Gun> for GunResponse {
    fn from(g: &Gun) -> Self {
        Self {
            id: g.id(),
            user_id: g.user_id(),
            catalog_id: g.catalog_id(),
            manufacturer: g.manufacturer().to_string(),
            model: g.model().to_string(),
            class: g.class().as_str().to_string(),
            caliber: g.caliber().to_string(),
            serial_last4: g.serial_last4(),
            date_of_purchase: g.date_of_purchase(),
            photo_url: g.photo_url().map(str::to_string),
            notes: g.notes().map(str::to_string),
            created_at: g.created_at(),
            updated_at: g.updated_at(),
        }
    }
}

/// Explicit response when the user wants to see the full serial.
#[derive(Debug, Serialize)]
pub struct GunSerialResponse {
    pub id: Uuid,
    pub serial: String,
}

impl GunSerialResponse {
    pub fn from_gun(g: &Gun) -> Self {
        Self { id: g.id(), serial: g.serial().expose_secret().to_string() }
    }
}

// ---------------- List query params ---------------- //

#[derive(Debug, Deserialize)]
pub struct GunListQuery {
    pub class: Option<WeaponClass>,
    pub caliber: Option<String>,
    pub q: Option<String>,
}

// ---------------- Catalog DTOs ---------------- //

#[derive(Debug, Deserialize)]
pub struct CreateCatalogRequest {
    pub manufacturer: String,
    pub model: String,
    pub class: WeaponClass,
    pub caliber: String,
    pub barrel_length_mm: Option<i32>,
    pub weight_g: Option<i32>,
    pub capacity: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateCatalogRequest {
    #[serde(default)] pub manufacturer: Option<String>,
    #[serde(default)] pub model: Option<String>,
    #[serde(default)] pub class: Option<WeaponClass>,
    #[serde(default)] pub caliber: Option<String>,
    #[serde(default)] pub barrel_length_mm: Option<Option<i32>>,
    #[serde(default)] pub weight_g: Option<Option<i32>>,
    #[serde(default)] pub capacity: Option<Option<i32>>,
    #[serde(default)] pub notes: Option<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct CatalogResponse {
    #[serde(flatten)]
    pub entry: CatalogEntry,
}

#[derive(Debug, Deserialize)]
pub struct CatalogListQuery {
    pub class: Option<WeaponClass>,
    pub caliber: Option<String>,
    pub q: Option<String>,
}
