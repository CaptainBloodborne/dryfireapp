//! Armory aggregate — the user's firearms.
//!
//! Two entities:
//!
//! - [`Gun`] — a single firearm owned by the user. The serial number
//!   is part of the entity but never travels in plaintext through the
//!   DB layer. It also never leaves
//!   the API in full unless the owner explicitly requests it.
//! - [`CatalogEntry`] — admin-curated reference of common models, used
//!   to autofill a `Gun` form.

use chrono::{DateTime, NaiveDate, Utc};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::errors::ValidationError;

// WeaponClass

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WeaponClass {
    Rifle,
    Shotgun,
    Pistol,
    Revolver,
    Smg,
    Other,
}

impl WeaponClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            WeaponClass::Rifle    => "rifle",
            WeaponClass::Shotgun  => "shotgun",
            WeaponClass::Pistol   => "pistol",
            WeaponClass::Revolver => "revolver",
            WeaponClass::Smg      => "smg",
            WeaponClass::Other    => "other",
        }
    }
}

impl std::str::FromStr for WeaponClass {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "rifle"    => Ok(WeaponClass::Rifle),
            "shotgun"  => Ok(WeaponClass::Shotgun),
            "pistol"   => Ok(WeaponClass::Pistol),
            "revolver" => Ok(WeaponClass::Revolver),
            "smg"      => Ok(WeaponClass::Smg),
            "other"    => Ok(WeaponClass::Other),
            other      => Err(ValidationError::Custom(
                format!("unknown weapon class `{other}`"))),
        }
    }
}

// Gun

/// A single firearm owned by the user.
///
/// Construct via [`Gun::register`] (new entry) or [`Gun::rehydrate`]
/// (loaded from DB). The `serial` field is `SecretString` so it can't
/// be accidentally logged.
#[derive(Debug, Clone)]
pub struct Gun {
    id: Uuid,
    user_id: Uuid,
    catalog_id: Option<Uuid>,
    manufacturer: String,
    model: String,
    class: WeaponClass,
    caliber: String,
    serial: SecretString,
    date_of_purchase: NaiveDate,
    photo_url: Option<String>,
    notes: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Gun {
    /// Smart constructor for a fresh registration. Validates that the
    /// trivially-checkable fields are non-empty.
    #[allow(clippy::too_many_arguments)]
    pub fn register(
        user_id: Uuid,
        catalog_id: Option<Uuid>,
        manufacturer: String,
        model: String,
        class: WeaponClass,
        caliber: String,
        serial: SecretString,
        date_of_purchase: NaiveDate,
        photo_url: Option<String>,
        notes: Option<String>,
    ) -> Result<Self, ValidationError> {
        if manufacturer.trim().is_empty() {
            return Err(ValidationError::Custom("manufacturer is required".into()));
        }
        if model.trim().is_empty() {
            return Err(ValidationError::Custom("model is required".into()));
        }
        if caliber.trim().is_empty() {
            return Err(ValidationError::Custom("caliber is required".into()));
        }
        let trimmed_serial = serial.expose_secret().trim();
        if trimmed_serial.is_empty() {
            return Err(ValidationError::Custom("serial is required".into()));
        }
        if trimmed_serial.len() > 64 {
            return Err(ValidationError::Custom("serial too long (>64)".into()));
        }

        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            user_id,
            catalog_id,
            manufacturer: manufacturer.trim().to_string(),
            model: model.trim().to_string(),
            class,
            caliber: caliber.trim().to_string(),
            serial: SecretString::from(trimmed_serial.to_string()),
            date_of_purchase,
            photo_url,
            notes,
            created_at: now,
            updated_at: now,
        })
    }

    /// Rehydrate from DB. Bypasses validation.
    #[allow(clippy::too_many_arguments)]
    pub fn rehydrate(
        id: Uuid,
        user_id: Uuid,
        catalog_id: Option<Uuid>,
        manufacturer: String,
        model: String,
        class: WeaponClass,
        caliber: String,
        serial: SecretString,
        date_of_purchase: NaiveDate,
        photo_url: Option<String>,
        notes: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id, user_id, catalog_id, manufacturer, model, class, caliber,
            serial, date_of_purchase, photo_url, notes,
            created_at, updated_at,
        }
    }

    // ---- accessors ---- //
    pub fn id(&self) -> Uuid { self.id }
    pub fn user_id(&self) -> Uuid { self.user_id }
    pub fn catalog_id(&self) -> Option<Uuid> { self.catalog_id }
    pub fn manufacturer(&self) -> &str { &self.manufacturer }
    pub fn model(&self) -> &str { &self.model }
    pub fn class(&self) -> WeaponClass { self.class }
    pub fn caliber(&self) -> &str { &self.caliber }
    pub fn serial(&self) -> &SecretString { &self.serial }
    pub fn date_of_purchase(&self) -> NaiveDate { self.date_of_purchase }
    pub fn photo_url(&self) -> Option<&str> { self.photo_url.as_deref() }
    pub fn notes(&self) -> Option<&str> { self.notes.as_deref() }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
    pub fn updated_at(&self) -> DateTime<Utc> { self.updated_at }

    /// Last 4 chars of the serial, plaintext — for UI hints
    /// ("which gun is …1234?") without revealing the full number.
    pub fn serial_last4(&self) -> String {
        let s = self.serial.expose_secret();
        let n = s.chars().count();
        if n <= 4 {
            s.to_string()
        } else {
            s.chars().skip(n - 4).collect()
        }
    }

    // mutators

    /// Apply a partial update. Caller passes Some(_) for each field to
    /// change. Returns the now-updated entity.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_update(
        &mut self,
        catalog_id: Option<Option<Uuid>>,
        manufacturer: Option<String>,
        model: Option<String>,
        class: Option<WeaponClass>,
        caliber: Option<String>,
        date_of_purchase: Option<NaiveDate>,
        photo_url: Option<Option<String>>,
        notes: Option<Option<String>>,
    ) -> Result<(), ValidationError> {
        if let Some(c) = catalog_id { self.catalog_id = c; }
        if let Some(m) = manufacturer {
            if m.trim().is_empty() {
                return Err(ValidationError::Custom("manufacturer cannot be empty".into()));
            }
            self.manufacturer = m.trim().into();
        }
        if let Some(m) = model {
            if m.trim().is_empty() {
                return Err(ValidationError::Custom("model cannot be empty".into()));
            }
            self.model = m.trim().into();
        }
        if let Some(c) = class { self.class = c; }
        if let Some(c) = caliber {
            if c.trim().is_empty() {
                return Err(ValidationError::Custom("caliber cannot be empty".into()));
            }
            self.caliber = c.trim().into();
        }
        if let Some(d) = date_of_purchase { self.date_of_purchase = d; }
        if let Some(p) = photo_url { self.photo_url = p; }
        if let Some(n) = notes { self.notes = n; }
        self.updated_at = Utc::now();
        Ok(())
    }
}

// CatalogEntry

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: Uuid,
    pub manufacturer: String,
    pub model: String,
    pub class: WeaponClass,
    pub caliber: String,
    pub barrel_length_mm: Option<i32>,
    pub weight_g: Option<i32>,
    pub capacity: Option<i32>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CatalogEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        manufacturer: String,
        model: String,
        class: WeaponClass,
        caliber: String,
        barrel_length_mm: Option<i32>,
        weight_g: Option<i32>,
        capacity: Option<i32>,
        notes: Option<String>,
    ) -> Result<Self, ValidationError> {
        if manufacturer.trim().is_empty()
           || model.trim().is_empty()
           || caliber.trim().is_empty() {
            return Err(ValidationError::Custom(
                "manufacturer, model, caliber are required".into()));
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            manufacturer: manufacturer.trim().into(),
            model: model.trim().into(),
            class,
            caliber: caliber.trim().into(),
            barrel_length_mm,
            weight_g,
            capacity,
            notes,
            created_at: now,
            updated_at: now,
        })
    }
}

// tests

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_gun() -> Gun {
        Gun::register(
            Uuid::new_v4(), None,
            "Sako".into(), "TRG-22".into(),
            WeaponClass::Rifle, ".308 Win".into(),
            SecretString::from("ABC123456".to_string()),
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            None, None,
        ).unwrap()
    }

    #[test]
    fn register_validates_required() {
        let e = Gun::register(
            Uuid::new_v4(), None,
            "".into(), "TRG-22".into(),
            WeaponClass::Rifle, ".308 Win".into(),
            SecretString::from("X".to_string()),
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            None, None,
        ).unwrap_err();
        assert!(matches!(e, ValidationError::Custom(_)));
    }

    #[test]
    fn serial_last4_basic() {
        let g = sample_gun();
        assert_eq!(g.serial_last4(), "3456");
    }

    #[test]
    fn serial_last4_short_serial() {
        let g = Gun::register(
            Uuid::new_v4(), None,
            "X".into(), "Y".into(), WeaponClass::Pistol, ".22 LR".into(),
            SecretString::from("AB".to_string()),
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            None, None,
        ).unwrap();
        assert_eq!(g.serial_last4(), "AB");
    }

    #[test]
    fn weapon_class_round_trips() {
        for c in [WeaponClass::Rifle, WeaponClass::Shotgun, WeaponClass::Pistol,
                  WeaponClass::Revolver, WeaponClass::Smg, WeaponClass::Other] {
            let s = c.as_str();
            let back: WeaponClass = s.parse().unwrap();
            assert_eq!(c, back);
        }
        assert!("crossbow".parse::<WeaponClass>().is_err());
    }

    #[test]
    fn apply_update_partial() {
        let mut g = sample_gun();
        let before = g.updated_at();
        std::thread::sleep(std::time::Duration::from_millis(2));
        g.apply_update(
            None, None,
            Some("TRG-42".into()),
            None, None, None, None, None,
        ).unwrap();
        assert_eq!(g.model(), "TRG-42");
        assert!(g.updated_at() > before);
        // Other fields untouched.
        assert_eq!(g.manufacturer(), "Sako");
    }

    #[test]
    fn apply_update_rejects_empty_string() {
        let mut g = sample_gun();
        let e = g.apply_update(
            None,
            Some("".into()),
            None, None, None, None, None, None,
        ).unwrap_err();
        assert!(matches!(e, ValidationError::Custom(_)));
    }
}
