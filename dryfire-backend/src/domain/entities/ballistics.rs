//! A named, saved combination of Bullet + Sight + Atmosphere defaults.
//! Tied to a specific user; optionally to a (gun_id, ammo_id) pair so
//! the same profile can be quickly re-selected for "my .308 + 175 SMK".

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::services::ballistics::{Atmosphere, Bullet, Sight};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallisticProfile {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    /// Optional linkage to an armoury record
    pub gun_id: Option<Uuid>,
    /// Optional linkage to an ammo record.
    pub ammo_id: Option<Uuid>,
    pub bullet: Bullet,
    pub sight: Sight,
    pub default_atmosphere: Atmosphere,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BallisticProfile {
    pub fn new(user_id: Uuid, name: String, bullet: Bullet, sight: Sight) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            name,
            gun_id: None,
            ammo_id: None,
            bullet,
            sight,
            default_atmosphere: Atmosphere::default(),
            created_at: now,
            updated_at: now,
        }
    }
}
