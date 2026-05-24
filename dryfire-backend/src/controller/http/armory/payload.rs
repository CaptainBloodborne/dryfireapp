// src/controller/http/armory/payload.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateGunRequest {
    pub manufacturer: String,
    pub model: String,
    pub serial: String,
    pub class: String,
    pub caliber: String,
    pub date_of_purchase: DateTime<Utc>,
    pub photo_url: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GunResponse {
    pub id: Uuid,
    pub manufacturer: String,
    pub model: String,
    pub serial_last4: String,
    pub class: String,
    pub caliber: String,
    pub date_of_purchase: DateTime<Utc>,
    pub photo_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<&crate::domain::entities::armory::Gun> for GunResponse {
    fn from(g: &crate::domain::entities::armory::Gun) -> Self {
        let serial = g.serial();
        let last4 = if serial.len() >= 4 {
            format!("***{}", &serial[serial.len() - 4..])
        } else { "***".into() };
        Self {
            id: g.id(),
            manufacturer: g.manufacturer().into(),
            model: g.model().into(),
            serial_last4: last4,
            class: g.class().as_str().into(),
            caliber: g.caliber().as_str().into(),
            date_of_purchase: g.date_of_purchase(),
            photo_url: g.photo_url().map(str::to_string),
            notes: g.notes().map(str::to_string),
            created_at: g.created_at(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PageRequest {
    #[serde(default = "default_page")]   pub page: u32,
    #[serde(default = "default_size")]   pub size: u32,
    pub sort: Option<String>,
    pub q:    Option<String>,
}
fn default_page() -> u32 { 1 }
fn default_size() -> u32 { 20 }

impl From<PageRequest> for crate::domain::repositories::armory::PageQuery {
    fn from(p: PageRequest) -> Self {
        Self { page: p.page, size: p.size, sort: p.sort, filter_text: p.q }
    }
}

#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub size: u32,
}

// --- ammo --- //
#[derive(Debug, Deserialize)]
pub struct CreateAmmoLotRequest {
    pub manufacturer: String,
    pub caliber: String,
    pub bullet_type: String,
    pub shell_type: String,
    pub bullet_weight_grains: Option<f64>,
    pub powder_charge_grains: Option<f64>,
    pub initial_quantity: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AmmoLotResponse {
    pub id: Uuid,
    pub manufacturer: String,
    pub caliber: String,
    pub bullet_type: String,
    pub shell_type: String,
    pub quantity_on_hand: i64,
    pub bullet_weight_grains: Option<f64>,
    pub powder_charge_grains: Option<f64>,
    pub notes: Option<String>,
}

impl From<&crate::domain::entities::armory::AmmoLot> for AmmoLotResponse {
    fn from(l: &crate::domain::entities::armory::AmmoLot) -> Self {
        Self {
            id: l.id,
            manufacturer: l.manufacturer.clone(),
            caliber: l.caliber.as_str().into(),
            bullet_type: l.bullet_type.as_str().into(),
            shell_type: l.shell_type.as_str().into(),
            quantity_on_hand: l.quantity_on_hand,
            bullet_weight_grains: l.bullet_weight_grains,
            powder_charge_grains: l.powder_charge_grains,
            notes: l.notes.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RecordTxnRequest {
    pub lot_id: Uuid,
    pub gun_id: Option<Uuid>,
    pub kind: String,                 // "purchase" | "use" | "adjust" | "loss"
    pub quantity: i64,                // positive
    pub happened_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}