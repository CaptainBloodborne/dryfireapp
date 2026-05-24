use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::errors::{DomainError, ValidationError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponClass {
    Pistol, Revolver, Rifle, Carbine, Shotgun,
    Smoothbore, AirGun, Traumatic, Other,
}

impl WeaponClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pistol => "pistol", Self::Revolver => "revolver",
            Self::Rifle => "rifle",   Self::Carbine => "carbine",
            Self::Shotgun => "shotgun", Self::Smoothbore => "smoothbore",
            Self::AirGun => "air_gun", Self::Traumatic => "traumatic",
            Self::Other => "other",
        }
    }
}
impl FromStr for WeaponClass {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "pistol" => Self::Pistol, "revolver" => Self::Revolver,
            "rifle" => Self::Rifle,   "carbine" => Self::Carbine,
            "shotgun" => Self::Shotgun, "smoothbore" => Self::Smoothbore,
            "air_gun" => Self::AirGun, "traumatic" => Self::Traumatic,
            "other" => Self::Other,
            o => return Err(ValidationError::Custom(format!("weapon_class `{o}`"))),
        })
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caliber(String);
impl Caliber {
    pub fn parse(raw: impl Into<String>) -> Result<Self, ValidationError> {
        let s = raw.into().trim().to_ascii_uppercase().replace(' ', "");
        if s.is_empty() || s.len() > 32 {
            return Err(ValidationError::Custom("caliber length".into()));
        }
        Ok(Self(s))
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

/// A user-owned firearm. Construct with [`Gun::register`].
#[derive(Debug, Clone)]
pub struct Gun {
    id: Uuid,
    owner_id: Uuid,
    manufacturer: String,
    model: String,
    serial: String,
    class: WeaponClass,
    caliber: Caliber,
    date_of_purchase: DateTime<Utc>,
    photo_url: Option<String>,
    notes: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Gun {
    #[allow(clippy::too_many_arguments)]
    pub fn register(
        owner_id: Uuid,
        manufacturer: String,
        model: String,
        serial: String,
        class: WeaponClass,
        caliber: Caliber,
        date_of_purchase: DateTime<Utc>,
        photo_url: Option<String>,
        notes: Option<String>,
    ) -> Result<Self, DomainError> {
        if manufacturer.trim().is_empty() || model.trim().is_empty() {
            return Err(DomainError::Validation(ValidationError::Custom(
                "manufacturer/model required".into(),
            )));
        }
        if serial.trim().is_empty() {
            return Err(DomainError::Validation(ValidationError::Custom(
                "serial required".into(),
            )));
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            owner_id, manufacturer, model, serial, class, caliber,
            date_of_purchase, photo_url, notes,
            created_at: now, updated_at: now,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rehydrate(
        id: Uuid, owner_id: Uuid,
        manufacturer: String, model: String, serial: String,
        class: WeaponClass, caliber: Caliber,
        date_of_purchase: DateTime<Utc>,
        photo_url: Option<String>, notes: Option<String>,
        created_at: DateTime<Utc>, updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id, owner_id, manufacturer, model, serial, class, caliber,
            date_of_purchase, photo_url, notes, created_at, updated_at,
        }
    }

    pub fn id(&self) -> Uuid { self.id }
    pub fn owner_id(&self) -> Uuid { self.owner_id }
    pub fn manufacturer(&self) -> &str { &self.manufacturer }
    pub fn model(&self) -> &str { &self.model }
    pub fn serial(&self) -> &str { &self.serial }
    pub fn class(&self) -> WeaponClass { self.class }
    pub fn caliber(&self) -> &Caliber { &self.caliber }
    pub fn date_of_purchase(&self) -> DateTime<Utc> { self.date_of_purchase }
    pub fn photo_url(&self) -> Option<&str> { self.photo_url.as_deref() }
    pub fn notes(&self) -> Option<&str> { self.notes.as_deref() }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
    pub fn updated_at(&self) -> DateTime<Utc> { self.updated_at }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BulletType {
    Fmj, Hp, Sp, Match, Tracer, Blank, Slug, Buckshot, Birdshot, Other,
}
impl BulletType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fmj=>"fmj",Self::Hp=>"hp",Self::Sp=>"sp",Self::Match=>"match",
            Self::Tracer=>"tracer",Self::Blank=>"blank",Self::Slug=>"slug",
            Self::Buckshot=>"buckshot",Self::Birdshot=>"birdshot",Self::Other=>"other",
        }
    }
}
impl FromStr for BulletType {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "fmj"=>Self::Fmj,"hp"=>Self::Hp,"sp"=>Self::Sp,"match"=>Self::Match,
            "tracer"=>Self::Tracer,"blank"=>Self::Blank,"slug"=>Self::Slug,
            "buckshot"=>Self::Buckshot,"birdshot"=>Self::Birdshot,"other"=>Self::Other,
            o => return Err(ValidationError::Custom(format!("bullet_type `{o}`"))),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellType { Brass, Steel, Aluminum, Polymer, Paper, Other }
impl ShellType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Brass=>"brass",Self::Steel=>"steel",Self::Aluminum=>"aluminum",
            Self::Polymer=>"polymer",Self::Paper=>"paper",Self::Other=>"other",
        }
    }
}
impl FromStr for ShellType {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "brass"=>Self::Brass,"steel"=>Self::Steel,"aluminum"=>Self::Aluminum,
            "polymer"=>Self::Polymer,"paper"=>Self::Paper,"other"=>Self::Other,
            o => return Err(ValidationError::Custom(format!("shell_type `{o}`"))),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AmmoLot {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub manufacturer: String,
    pub caliber: Caliber,
    pub bullet_type: BulletType,
    pub shell_type: ShellType,
    pub bullet_weight_grains: Option<f64>,
    pub powder_charge_grains: Option<f64>,
    pub quantity_on_hand: i64,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmmoTxnKind { Purchase, Use, Adjust, Loss }
impl AmmoTxnKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Purchase=>"purchase",Self::Use=>"use",
            Self::Adjust=>"adjust",Self::Loss=>"loss",
        }
    }
}
impl FromStr for AmmoTxnKind {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "purchase"=>Self::Purchase,"use"=>Self::Use,
            "adjust"=>Self::Adjust,"loss"=>Self::Loss,
            o => return Err(ValidationError::Custom(format!("txn_kind `{o}`"))),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AmmoTransaction {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub lot_id: Uuid,
    pub gun_id: Option<Uuid>,
    pub kind: AmmoTxnKind,
    pub delta: i64,
    pub happened_at: DateTime<Utc>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}