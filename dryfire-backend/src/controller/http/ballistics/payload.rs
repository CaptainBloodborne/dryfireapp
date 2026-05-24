use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::entities::ballistics::{
    AdjustmentUnit, BallisticInput, BallisticProfile, Environment, TrajectoryPoint,
};

#[derive(Debug, Deserialize)]
pub struct ComputeTrajectoryRequest {
    pub input: BallisticInput,
    #[serde(default)]
    pub env: Environment,
    /// "moa_shooter" (default) | "moa_true" | "mil"
    #[serde(default = "default_unit")]
    pub unit: String,
    #[serde(default = "default_step")]
    pub step_m: f64,
    #[serde(default = "default_max")]
    pub max_range_m: f64,
}
fn default_unit() -> String { "moa_shooter".into() }
fn default_step() -> f64 { 50.0 }
fn default_max() -> f64 { 1000.0 }

#[derive(Debug, Serialize)]
pub struct ComputeTrajectoryResponse {
    pub unit: String,
    pub points: Vec<TrajectoryPoint>,
}

#[derive(Debug, Deserialize)]
pub struct SaveProfileRequest {
    pub name: String,
    pub gun_id: Option<Uuid>,
    pub lot_id: Option<Uuid>,
    pub input: BallisticInput,
}

#[derive(Debug, Serialize)]
pub struct ProfileResponse {
    pub id: Uuid,
    pub name: String,
    pub gun_id: Option<Uuid>,
    pub lot_id: Option<Uuid>,
    pub input: BallisticInput,
}

impl From<&BallisticProfile> for ProfileResponse {
    fn from(p: &BallisticProfile) -> Self {
        Self {
            id: p.id,
            name: p.name.clone(),
            gun_id: p.gun_id,
            lot_id: p.lot_id,
            input: p.input.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    /// "csv" or "json"; default "json".
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "super::payload::default_unit")]
    pub unit: String,
    #[serde(default = "super::payload::default_step")]
    pub step_m: f64,
    #[serde(default = "super::payload::default_max")]
    pub max_range_m: f64,
}
fn default_format() -> String { "json".into() }

/// Format a `Vec<TrajectoryPoint>` as RFC 4180 CSV.
pub fn points_to_csv(points: &[TrajectoryPoint]) -> String {
    let mut s = String::with_capacity(points.len() * 96);
    s.push_str("distance_m,drop_cm,drift_cm,velocity_mps,energy_j,time_s,elevation_units,windage_units\n");
    for p in points {
        use std::fmt::Write;
        let _ = writeln!(
            s,
            "{:.1},{:.3},{:.3},{:.2},{:.2},{:.4},{:.3},{:.3}",
            p.distance_m, p.drop_cm, p.drift_cm,
            p.velocity_mps, p.energy_j, p.time_s,
            p.elevation_units, p.windage_units,
        );
    }
    s
}