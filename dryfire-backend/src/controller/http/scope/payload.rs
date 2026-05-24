// src/controller/http/scope/payload.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::{
    ballistics::AdjustmentUnit,
    scope::{ScopeProfile, ZeroRequest, ZeroResponse},
};

#[derive(Debug, Deserialize)]
pub struct ComputeZeroRequest {
    pub vertical_cm: f64,
    pub horizontal_cm: f64,
    pub distance_m: f64,
    pub unit: String,           // "moa_shooter" | "moa_true" | "mil"
    pub click_value: f64,
}

#[derive(Debug, Serialize)]
pub struct ComputeZeroResponse {
    pub elevation_clicks: i32,
    pub windage_clicks: i32,
    pub elevation_units: f64,
    pub windage_units: f64,
}

impl From<ZeroResponse> for ComputeZeroResponse {
    fn from(r: ZeroResponse) -> Self {
        Self {
            elevation_clicks: r.elevation_clicks,
            windage_clicks: r.windage_clicks,
            elevation_units: r.elevation_units,
            windage_units: r.windage_units,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SaveScopeProfileRequest {
    pub name: String,
    pub gun_id: Option<Uuid>,
    pub unit: String,
    pub click_value: f64,
    pub elevation_max_clicks: Option<i32>,
    pub windage_max_clicks: Option<i32>,
    #[serde(default = "default_mount")]
    pub mount_height_mm: f64,
}
fn default_mount() -> f64 { 38.0 }

#[derive(Debug, Serialize)]
pub struct ScopeProfileResponse {
    pub id: Uuid,
    pub name: String,
    pub gun_id: Option<Uuid>,
    pub unit: String,
    pub click_value: f64,
    pub elevation_max_clicks: Option<i32>,
    pub windage_max_clicks: Option<i32>,
    pub mount_height_mm: f64,
}

impl From<&ScopeProfile> for ScopeProfileResponse {
    fn from(p: &ScopeProfile) -> Self {
        Self {
            id: p.id,
            name: p.name.clone(),
            gun_id: p.gun_id,
            unit: p.unit.as_str().into(),
            click_value: p.click_value,
            elevation_max_clicks: p.elevation_max_clicks,
            windage_max_clicks: p.windage_max_clicks,
            mount_height_mm: p.mount_height_mm,
        }
    }
}

pub fn parse_zero_request(req: ComputeZeroRequest) -> Result<ZeroRequest, crate::domain::errors::ValidationError> {
    let unit: AdjustmentUnit = req.unit.parse()?;
    Ok(ZeroRequest {
        vertical_cm: req.vertical_cm,
        horizontal_cm: req.horizontal_cm,
        distance_m: req.distance_m,
        unit,
        click_value: req.click_value,
    })
}