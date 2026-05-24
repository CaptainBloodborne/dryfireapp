// src/domain/entities/ballistics.rs

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::errors::ValidationError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustmentUnit {
    /// Shooter MOA: 1″ at 100 yd.
    MoaShooter,
    /// True MOA: 1.047″ at 100 yd.
    MoaTrue,
    /// Milliradian: 1 cm at 100 m × 10.
    Mil,
}
impl AdjustmentUnit {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MoaShooter=>"moa_shooter",Self::MoaTrue=>"moa_true",Self::Mil=>"mil",
        }
    }
    /// Linear size (metres) of one unit at one metre of range.
    pub fn metres_at_one_metre(&self) -> f64 {
        match self {
            Self::MoaShooter => 2.54e-4,    // 1″/100yd ≈ 0.000254 m / m
            Self::MoaTrue    => 2.908e-4,   // 1.047″/100yd
            Self::Mil        => 1.0e-3,     // 0.001 rad ≈ 0.001 m / m
        }
    }
}
impl FromStr for AdjustmentUnit {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "moa_shooter"|"moa"=>Self::MoaShooter,
            "moa_true"=>Self::MoaTrue,
            "mil"|"mrad"=>Self::Mil,
            o => return Err(ValidationError::Custom(format!("unit `{o}`"))),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallisticInput {
    pub caliber: String,
    pub bullet_weight_grains: f64,
    pub muzzle_velocity_mps: f64,
    pub ballistic_coefficient: f64,     // G1, dimensionless
    pub sight_height_mm: f64,
    pub zero_distance_m: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Environment {
    pub temperature_c: Option<f64>,     // default 15
    pub pressure_hpa: Option<f64>,      // default 1013.25
    pub humidity_pct: Option<f64>,      // default 50
    pub wind_speed_mps: Option<f64>,    // default 0
    pub wind_direction_deg: Option<f64>,// from where: 0=12 o'clock, 90=3 o'clock
}

#[derive(Debug, Clone, Serialize)]
pub struct TrajectoryPoint {
    pub distance_m: f64,
    pub drop_cm: f64,            // negative = below LOS
    pub drift_cm: f64,           // positive = right
    pub velocity_mps: f64,
    pub energy_j: f64,
    pub time_s: f64,
    /// Up/down click correction relative to the zero distance.
    pub elevation_units: f64,    // in requested unit
    pub windage_units: f64,
}

#[derive(Debug, Clone)]
pub struct BallisticProfile {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub gun_id: Option<Uuid>,
    pub lot_id: Option<Uuid>,
    pub input: BallisticInput,
}