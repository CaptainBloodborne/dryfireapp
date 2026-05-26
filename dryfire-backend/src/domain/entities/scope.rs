//! Persistent scope/optic configuration. One per optic the user owns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::services::scope::{AdjustmentUnit, ClickValue};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    /// Optional: tie this optic to a specific Gun.
    pub gun_id: Option<Uuid>,
    pub name: String,
    pub unit: AdjustmentUnit,
    pub click_value: ClickValue,
    /// Max travel of the elevation turret, in adjustment units. Used to
    /// warn when the requested dial exceeds the optic's range.
    pub max_elevation_units: Option<f64>,
    pub max_windage_units: Option<f64>,
    pub mount_height_mm: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ScopeProfile {
    pub fn new(
        user_id: Uuid,
        name: String,
        unit: AdjustmentUnit,
        click_value: ClickValue,
        mount_height_mm: f64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            gun_id: None,
            name,
            unit,
            click_value,
            max_elevation_units: None,
            max_windage_units: None,
            mount_height_mm,
            created_at: now,
            updated_at: now,
        }
    }
}
