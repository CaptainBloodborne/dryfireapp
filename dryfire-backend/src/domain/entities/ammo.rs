//! Ammo aggregate. Three types here:
//!
//! - [`AmmoType`] — catalog entry: manufacturer - caliber - bullet type.
//! - [`AmmoStock`] — current on-hand quantity for a (user, ammo_type)
//!   pair. A materialized aggregate of the transaction log.
//! - [`AmmoTransaction`] — immutable ledger row.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::errors::ValidationError;

// enums

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BulletType {
    Fmj, Jhp, Sp, Lrn, Wad, Match,
    Slug, Buckshot, Birdshot, Other,
}

impl BulletType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BulletType::Fmj      => "fmj",
            BulletType::Jhp      => "jhp",
            BulletType::Sp       => "sp",
            BulletType::Lrn      => "lrn",
            BulletType::Wad      => "wad",
            BulletType::Match    => "match",
            BulletType::Slug     => "slug",
            BulletType::Buckshot => "buckshot",
            BulletType::Birdshot => "birdshot",
            BulletType::Other    => "other",
        }
    }
}

impl std::str::FromStr for BulletType {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "fmj"      => Ok(BulletType::Fmj),
            "jhp"      => Ok(BulletType::Jhp),
            "sp"       => Ok(BulletType::Sp),
            "lrn"      => Ok(BulletType::Lrn),
            "wad"      => Ok(BulletType::Wad),
            "match"    => Ok(BulletType::Match),
            "slug"     => Ok(BulletType::Slug),
            "buckshot" => Ok(BulletType::Buckshot),
            "birdshot" => Ok(BulletType::Birdshot),
            "other"    => Ok(BulletType::Other),
            other      => Err(ValidationError::Custom(
                format!("unknown bullet_type `{other}`"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectileType {
    Centerfire, Rimfire, Shotshell, Blank, Other,
}

impl ProjectileType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectileType::Centerfire => "centerfire",
            ProjectileType::Rimfire    => "rimfire",
            ProjectileType::Shotshell  => "shotshell",
            ProjectileType::Blank      => "blank",
            ProjectileType::Other      => "other",
        }
    }
}

impl std::str::FromStr for ProjectileType {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "centerfire" => Ok(ProjectileType::Centerfire),
            "rimfire"    => Ok(ProjectileType::Rimfire),
            "shotshell"  => Ok(ProjectileType::Shotshell),
            "blank"      => Ok(ProjectileType::Blank),
            "other"      => Ok(ProjectileType::Other),
            other        => Err(ValidationError::Custom(
                format!("unknown projectile_type `{other}`"))),
        }
    }
}

// AmmoType

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmoType {
    pub id: Uuid,
    pub manufacturer: String,
    pub name: String,
    pub caliber: String,
    pub bullet_type: BulletType,
    pub projectile_type: ProjectileType,
    pub powder_charge_grain: Option<f64>,
    pub bullet_weight_grain: Option<f64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AmmoType {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        manufacturer: String,
        name: String,
        caliber: String,
        bullet_type: BulletType,
        projectile_type: ProjectileType,
        powder_charge_grain: Option<f64>,
        bullet_weight_grain: Option<f64>,
        notes: Option<String>,
    ) -> Result<Self, ValidationError> {
        if manufacturer.trim().is_empty()
            || name.trim().is_empty()
            || caliber.trim().is_empty()
        {
            return Err(ValidationError::Custom(
                "manufacturer, name, caliber are required".into()));
        }
        if let Some(w) = powder_charge_grain {
            if w <= 0.0 {
                return Err(ValidationError::Custom(
                    "powder_charge_grain must be positive".into()));
            }
        }
        if let Some(w) = bullet_weight_grain {
            if w <= 0.0 {
                return Err(ValidationError::Custom(
                    "bullet_weight_grain must be positive".into()));
            }
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            manufacturer: manufacturer.trim().into(),
            name: name.trim().into(),
            caliber: caliber.trim().into(),
            bullet_type,
            projectile_type,
            powder_charge_grain,
            bullet_weight_grain,
            notes,
            created_at: now,
            updated_at: now,
        })
    }
}

// AmmoStock

#[derive(Debug, Clone, Serialize)]
pub struct AmmoStock {
    pub user_id: Uuid,
    pub ammo_type_id: Uuid,
    pub quantity: i32,
    pub updated_at: DateTime<Utc>,
}

// AmmoTransaction

#[derive(Debug, Clone, Serialize)]
pub struct AmmoTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub ammo_type_id: Uuid,
    pub gun_id: Option<Uuid>,
    /// Positive = acquired, negative = consumed.
    pub delta: i32,
    pub occurred_at: DateTime<Utc>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl AmmoTransaction {
    pub fn new(
        user_id: Uuid,
        ammo_type_id: Uuid,
        gun_id: Option<Uuid>,
        delta: i32,
        occurred_at: DateTime<Utc>,
        note: Option<String>,
    ) -> Result<Self, ValidationError> {
        if delta == 0 {
            return Err(ValidationError::Custom("delta must be non-zero".into()));
        }

        if !(-1_000_000..=1_000_000).contains(&delta) {
            return Err(ValidationError::Custom(
                "delta out of adequate range (|delta| > 1,000,000)".into()));
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            user_id,
            ammo_type_id,
            gun_id,
            delta,
            occurred_at,
            note,
            created_at: now,
        })
    }
}

// tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ammo_type_validates_required_fields() {
        let e = AmmoType::new(
            "".into(), "Match King".into(), ".308".into(),
            BulletType::Match, ProjectileType::Centerfire,
            None, Some(175.0), None,
        ).unwrap_err();
        assert!(matches!(e, ValidationError::Custom(_)));
    }

    #[test]
    fn ammo_type_rejects_nonpositive_weights() {
        let e = AmmoType::new(
            "Hornady".into(), "BTHP".into(), ".308".into(),
            BulletType::Match, ProjectileType::Centerfire,
            Some(0.0), None, None,
        ).unwrap_err();
        assert!(matches!(e, ValidationError::Custom(_)));
    }

    #[test]
    fn tx_rejects_zero_delta() {
        let e = AmmoTransaction::new(
            Uuid::new_v4(), Uuid::new_v4(), None, 0,
            Utc::now(), None,
        ).unwrap_err();
        assert!(matches!(e, ValidationError::Custom(_)));
    }

    #[test]
    fn tx_rejects_absurd_delta() {
        let e = AmmoTransaction::new(
            Uuid::new_v4(), Uuid::new_v4(), None, 10_000_000,
            Utc::now(), None,
        ).unwrap_err();
        assert!(matches!(e, ValidationError::Custom(_)));
    }

    #[test]
    fn bullet_type_round_trips() {
        for b in [
            BulletType::Fmj, BulletType::Jhp, BulletType::Sp, BulletType::Lrn,
            BulletType::Wad, BulletType::Match, BulletType::Slug,
            BulletType::Buckshot, BulletType::Birdshot, BulletType::Other,
        ] {
            let s = b.as_str();
            let back: BulletType = s.parse().unwrap();
            assert_eq!(b, back);
        }
        assert!("plasma".parse::<BulletType>().is_err());
    }
}
