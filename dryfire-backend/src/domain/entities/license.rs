//! Licenses aggregate — user permits / registrations / hunting cards.
//!
//! Two types:
//! - [`LicenseType`] — admin-managed reference. Carries the default validity period in days,
//!   so we can auto-compute `expires_at` for new licenses of that type.
//! - [`License`] — a user-owned document instance.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::errors::ValidationError;

// LicenseType

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseType {
    pub id: Uuid,
    /// Stable machine-readable code: `storage`, `hunting`, `self_defense`, …
    /// Clients use this to map to localised display strings.
    pub code: String,
    pub name: String,
    pub region: String,
    /// `None` means manual expiry only — we won't auto-compute.
    pub validity_days: Option<i32>,
    pub instructions: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LicenseType {
    pub fn new(
        code: String,
        name: String,
        region: String,
        validity_days: Option<i32>,
        instructions: Option<String>,
    ) -> Result<Self, ValidationError> {
        if code.trim().is_empty() {
            return Err(ValidationError::Custom("code is required".into()));
        }
        if name.trim().is_empty() {
            return Err(ValidationError::Custom("name is required".into()));
        }
        if let Some(d) = validity_days {
            if d <= 0 {
                return Err(ValidationError::Custom(
                    "validity_days must be positive".into()));
            }
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            code: code.trim().to_string(),
            name: name.trim().to_string(),
            region: region.trim().to_string(),
            validity_days,
            instructions,
            created_at: now,
            updated_at: now,
        })
    }
}

// License

#[derive(Debug, Clone)]
pub struct License {
    id: Uuid,
    user_id: Uuid,
    license_type_id: Option<Uuid>,
    license_number: String,
    issuer: String,
    issued_at: NaiveDate,
    expires_at: NaiveDate,
    notes: Option<String>,
    scan_url: Option<String>,
    /// Linked gun IDs. Loaded on demand by the repository.
    gun_ids: Vec<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl License {
    /// Build a fresh license. `expires_at` may be computed by the use
    /// case from `license_type.validity_days` when not supplied
    /// (see [`License::compute_expiry`]).
    #[allow(clippy::too_many_arguments)]
    pub fn register(
        user_id: Uuid,
        license_type_id: Option<Uuid>,
        license_number: String,
        issuer: String,
        issued_at: NaiveDate,
        expires_at: NaiveDate,
        notes: Option<String>,
        scan_url: Option<String>,
        gun_ids: Vec<Uuid>,
    ) -> Result<Self, ValidationError> {
        if license_number.trim().is_empty() {
            return Err(ValidationError::Custom("license_number is required".into()));
        }
        if issuer.trim().is_empty() {
            return Err(ValidationError::Custom("issuer is required".into()));
        }
        if expires_at < issued_at {
            return Err(ValidationError::Custom(
                "expires_at must be on or after issued_at".into()));
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            user_id,
            license_type_id,
            license_number: license_number.trim().to_string(),
            issuer: issuer.trim().to_string(),
            issued_at,
            expires_at,
            notes,
            scan_url,
            gun_ids,
            created_at: now,
            updated_at: now,
        })
    }

    /// Rehydrate from DB. Bypasses validation.
    #[allow(clippy::too_many_arguments)]
    pub fn rehydrate(
        id: Uuid,
        user_id: Uuid,
        license_type_id: Option<Uuid>,
        license_number: String,
        issuer: String,
        issued_at: NaiveDate,
        expires_at: NaiveDate,
        notes: Option<String>,
        scan_url: Option<String>,
        gun_ids: Vec<Uuid>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id, user_id, license_type_id, license_number, issuer,
            issued_at, expires_at, notes, scan_url, gun_ids,
            created_at, updated_at,
        }
    }

    /// Compute an expiry date by adding `validity_days` to `issued_at`.
    /// Used by the use case when the user didn't supply `expires_at`
    /// but the chosen license type knows the validity period.
    pub fn compute_expiry(issued_at: NaiveDate, validity_days: i32) -> NaiveDate {
        issued_at + Duration::days(validity_days as i64)
    }

    // accessors
    pub fn id(&self) -> Uuid { self.id }
    pub fn user_id(&self) -> Uuid { self.user_id }
    pub fn license_type_id(&self) -> Option<Uuid> { self.license_type_id }
    pub fn license_number(&self) -> &str { &self.license_number }
    pub fn issuer(&self) -> &str { &self.issuer }
    pub fn issued_at(&self) -> NaiveDate { self.issued_at }
    pub fn expires_at(&self) -> NaiveDate { self.expires_at }
    pub fn notes(&self) -> Option<&str> { self.notes.as_deref() }
    pub fn scan_url(&self) -> Option<&str> { self.scan_url.as_deref() }
    pub fn gun_ids(&self) -> &[Uuid] { &self.gun_ids }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
    pub fn updated_at(&self) -> DateTime<Utc> { self.updated_at }

    /// True if the license has already expired by `today`.
    pub fn is_expired(&self, today: NaiveDate) -> bool {
        self.expires_at < today
    }

    /// Days remaining until expiry. Negative if already expired.
    pub fn days_until_expiry(&self, today: NaiveDate) -> i64 {
        (self.expires_at - today).num_days()
    }

    // mutators
    #[allow(clippy::too_many_arguments)]
    pub fn apply_update(
        &mut self,
        license_type_id: Option<Option<Uuid>>,
        license_number: Option<String>,
        issuer: Option<String>,
        issued_at: Option<NaiveDate>,
        expires_at: Option<NaiveDate>,
        notes: Option<Option<String>>,
        scan_url: Option<Option<String>>,
        gun_ids: Option<Vec<Uuid>>,
    ) -> Result<(), ValidationError> {
        if let Some(t) = license_type_id { self.license_type_id = t; }
        if let Some(n) = license_number {
            if n.trim().is_empty() {
                return Err(ValidationError::Custom("license_number cannot be empty".into()));
            }
            self.license_number = n.trim().into();
        }
        if let Some(i) = issuer {
            if i.trim().is_empty() {
                return Err(ValidationError::Custom("issuer cannot be empty".into()));
            }
            self.issuer = i.trim().into();
        }
        if let Some(d) = issued_at { self.issued_at = d; }
        if let Some(d) = expires_at { self.expires_at = d; }
        if self.expires_at < self.issued_at {
            return Err(ValidationError::Custom(
                "expires_at must be on or after issued_at".into()));
        }
        if let Some(n) = notes { self.notes = n; }
        if let Some(s) = scan_url { self.scan_url = s; }
        if let Some(g) = gun_ids { self.gun_ids = g; }
        self.updated_at = Utc::now();
        Ok(())
    }
}

// ---------------- tests ---------------- //

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_license(expires: NaiveDate) -> License {
        License::register(
            Uuid::new_v4(), None,
            "AB-12345".into(), "МВД РФ".into(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            expires,
            None, None, vec![],
        ).unwrap()
    }

    #[test]
    fn expires_at_must_be_after_issued_at() {
        let e = License::register(
            Uuid::new_v4(), None,
            "AB-1".into(), "Org".into(),
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 5, 1).unwrap(),
            None, None, vec![],
        ).unwrap_err();
        assert!(matches!(e, ValidationError::Custom(_)));
    }

    #[test]
    fn days_until_expiry() {
        let l = fake_license(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap());
        assert_eq!(l.days_until_expiry(NaiveDate::from_ymd_opt(2026, 5, 1).unwrap()), 31);
        assert_eq!(l.days_until_expiry(NaiveDate::from_ymd_opt(2026, 7, 1).unwrap()), -30);
    }

    #[test]
    fn compute_expiry_adds_days() {
        let issued = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let expiry = License::compute_expiry(issued, 1827); // 5 years
        assert_eq!(expiry, NaiveDate::from_ymd_opt(2029, 1, 1).unwrap());
    }

    #[test]
    fn is_expired_check() {
        let l = fake_license(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap());
        assert!(l.is_expired(NaiveDate::from_ymd_opt(2024, 6, 2).unwrap()));
        assert!(!l.is_expired(NaiveDate::from_ymd_opt(2024, 6, 1).unwrap()));
    }

    #[test]
    fn apply_update_validates_date_ordering() {
        let mut l = fake_license(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        let e = l.apply_update(
            None, None, None,
            Some(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()), // issued moves after expires
            None, None, None, None,
        ).unwrap_err();
        assert!(matches!(e, ValidationError::Custom(_)));
    }
}
