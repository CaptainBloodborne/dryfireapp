// src/domain/entities/scope.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ballistics::AdjustmentUnit;

#[derive(Debug, Clone)]
pub struct ScopeProfile {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub gun_id: Option<Uuid>,
    pub name: String,
    pub unit: AdjustmentUnit,
    /// Size of one click in `unit`'s native scale.
    /// E.g. for 1/4 MOA scopes → 0.25; for 0.1-MIL → 0.1.
    pub click_value: f64,
    pub elevation_max_clicks: Option<i32>,
    pub windage_max_clicks: Option<i32>,
    pub mount_height_mm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroRequest {
    /// Vertical observed point-of-impact deviation in cm (positive = high).
    pub vertical_cm: f64,
    /// Horizontal POI deviation in cm (positive = right).
    pub horizontal_cm: f64,
    pub distance_m: f64,
    pub unit: AdjustmentUnit,
    pub click_value: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ZeroResponse {
    /// Negative clicks = "down"; positive = "up".
    pub elevation_clicks: i32,
    /// Negative = "left"; positive = "right".
    pub windage_clicks: i32,
    /// Raw fractional clicks before rounding (for diagnostics).
    pub elevation_units: f64,
    pub windage_units: f64,
}