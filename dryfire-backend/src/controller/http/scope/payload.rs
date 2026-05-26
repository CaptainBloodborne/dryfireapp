use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{
    entities::scope::ScopeProfile,
    services::scope::{
        AdjustmentUnit, ClickValue, ClicksRequest, ClicksResponse,
        ReZeroRequest, ReZeroResponse,
    },
};

#[derive(Debug, Deserialize)]
pub struct ClicksHttpRequest(pub ClicksRequest);

#[derive(Debug, Serialize)]
pub struct ClicksHttpResponse(pub ClicksResponse);

#[derive(Debug, Deserialize)]
pub struct ReZeroHttpRequest(pub ReZeroRequest);

#[derive(Debug, Serialize)]
pub struct ReZeroHttpResponse(pub ReZeroResponse);

#[derive(Debug, Deserialize)]
pub struct CreateScopeProfileRequest {
    pub name: String,
    pub gun_id: Option<Uuid>,
    pub unit: AdjustmentUnit,
    pub click_value: ClickValue,
    pub max_elevation_units: Option<f64>,
    pub max_windage_units: Option<f64>,
    pub mount_height_mm: f64,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateScopeProfileRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub gun_id: Option<Option<Uuid>>,
    #[serde(default)]
    pub unit: Option<AdjustmentUnit>,
    #[serde(default)]
    pub click_value: Option<ClickValue>,
    #[serde(default)]
    pub max_elevation_units: Option<Option<f64>>,
    #[serde(default)]
    pub max_windage_units: Option<Option<f64>>,
    #[serde(default)]
    pub mount_height_mm: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct ScopeProfileResponse {
    #[serde(flatten)]
    pub profile: ScopeProfile,
}
