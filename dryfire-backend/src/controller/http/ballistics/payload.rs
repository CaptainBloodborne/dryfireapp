//! Wire-format DTOs for the ballistics routes.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{
    entities::ballistics::BallisticProfile,
    services::ballistics::{Atmosphere, Bullet, Sight, Wind},
};

// Compute trajectory

#[derive(Debug, Deserialize)]
pub struct ComputeTrajectoryRequest {
    pub bullet: Bullet,
    pub sight: Sight,
    #[serde(default)]
    pub atmosphere: Atmosphere,
    #[serde(default = "default_wind")]
    pub wind: Wind,
    pub steps_m: Vec<f64>,
}

fn default_wind() -> Wind { Wind { speed_mps: 0.0, from_clock: 12.0 } }

#[derive(Debug, Serialize)]
pub struct ComputeTrajectoryResponse {
    pub points: Vec<crate::domain::services::ballistics::TrajectoryPoint>,
}

// Profile DTOs

#[derive(Debug, Deserialize)]
pub struct CreateBallisticProfileRequest {
    pub name: String,
    pub gun_id: Option<Uuid>,
    pub ammo_id: Option<Uuid>,
    pub bullet: Bullet,
    pub sight: Sight,
    #[serde(default)]
    pub default_atmosphere: Atmosphere,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateBallisticProfileRequest {
    #[serde(default)]
    pub name: Option<String>,
    /// `None` = leave unchanged. `Some(None)` = clear. `Some(Some(x))` = set.
    #[serde(default)]
    pub gun_id: Option<Option<Uuid>>,
    #[serde(default)]
    pub ammo_id: Option<Option<Uuid>>,
    #[serde(default)]
    pub bullet: Option<Bullet>,
    #[serde(default)]
    pub sight: Option<Sight>,
    #[serde(default)]
    pub default_atmosphere: Option<Atmosphere>,
}

#[derive(Debug, Serialize)]
pub struct BallisticProfileResponse {
    #[serde(flatten)]
    pub profile: BallisticProfile,
}
