use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::domain::errors::{DomainError, ValidationError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseKind {
    Storage, Carry, Hunting, Sport, SelfDefense, Transport, Other,
}

impl LicenseKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Storage=>"storage",Self::Carry=>"carry",Self::Hunting=>"hunting",
            Self::Sport=>"sport",Self::SelfDefense=>"self_defense",
            Self::Transport=>"transport",Self::Other=>"other",
        }
    }
    /// Standard validity period (Russian regulation defaults).
    /// Returns `None` for kinds with no statutory default.
    pub fn default_validity(&self) -> Option<Duration> {
        match self {
            Self::Storage | Self::Carry | Self::SelfDefense
                => Some(Duration::days(365 * 5)),
            Self::Hunting => Some(Duration::days(365)),
            _ => None,
        }
    }
}

impl FromStr for LicenseKind {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "storage"=>Self::Storage,"carry"=>Self::Carry,"hunting"=>Self::Hunting,
            "sport"=>Self::Sport,"self_defense"=>Self::SelfDefense,
            "transport"=>Self::Transport,"other"=>Self::Other,
            o => return Err(ValidationError::Custom(format!("license_kind `{o}`"))),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseStatus { Active, Expired, Revoked }
impl LicenseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active=>"active",Self::Expired=>"expired",Self::Revoked=>"revoked",
        }
    }
}
impl FromStr for LicenseStatus {
    type Err = ValidationError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "active"=>Self::Active,"expired"=>Self::Expired,"revoked"=>Self::Revoked,
            o => return Err(ValidationError::Custom(format!("license_status `{o}`"))),
        })
    }
}

#[derive(Debug, Clone)]
pub struct License {
    id: Uuid,
    owner_id: Uuid,
    kind: LicenseKind,
    issuing_org: String,
    issued_at: NaiveDate,
    expires_at: NaiveDate,
    status: LicenseStatus,
    document_url: Option<String>,
    instructions: Option<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl License {
    /// Create a new license, auto-deriving `expires_at` from `kind`
    /// when the caller hasn't supplied it explicitly.
    #[allow(clippy::too_many_arguments)]
    pub fn issue(
        owner_id: Uuid,
        kind: LicenseKind,
        issuing_org: String,
        issued_at: NaiveDate,
        expires_at_override: Option<NaiveDate>,
        document_url: Option<String>,
        instructions: Option<String>,
    ) -> Result<Self, DomainError> {
        if issuing_org.trim().is_empty() {
            return Err(DomainError::Validation(ValidationError::Custom(
                "issuing_org required".into(),
            )));
        }
        let expires_at = expires_at_override
            .or_else(|| kind.default_validity().map(|d| issued_at + d))
            .ok_or_else(|| DomainError::Validation(ValidationError::Custom(
                "expires_at is required for this license kind".into(),
            )))?;
        if expires_at < issued_at {
            return Err(DomainError::Validation(ValidationError::Custom(
                "expires_at < issued_at".into(),
            )));
        }
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            owner_id, kind, issuing_org, issued_at, expires_at,
            status: LicenseStatus::Active,
            document_url, instructions,
            created_at: now, updated_at: now,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rehydrate(
        id: Uuid, owner_id: Uuid, kind: LicenseKind, issuing_org: String,
        issued_at: NaiveDate, expires_at: NaiveDate, status: LicenseStatus,
        document_url: Option<String>, instructions: Option<String>,
        created_at: chrono::DateTime<Utc>, updated_at: chrono::DateTime<Utc>,
    ) -> Self {
        Self {
            id, owner_id, kind, issuing_org, issued_at, expires_at, status,
            document_url, instructions, created_at, updated_at,
        }
    }

    pub fn id(&self) -> Uuid { self.id }
    pub fn owner_id(&self) -> Uuid { self.owner_id }
    pub fn kind(&self) -> LicenseKind { self.kind }
    pub fn issuing_org(&self) -> &str { &self.issuing_org }
    pub fn issued_at(&self) -> NaiveDate { self.issued_at }
    pub fn expires_at(&self) -> NaiveDate { self.expires_at }
    pub fn status(&self) -> LicenseStatus { self.status }
    pub fn document_url(&self) -> Option<&str> { self.document_url.as_deref() }
    pub fn instructions(&self) -> Option<&str> { self.instructions.as_deref() }
    pub fn created_at(&self) -> chrono::DateTime<Utc> { self.created_at }
    pub fn updated_at(&self) -> chrono::DateTime<Utc> { self.updated_at }

    pub fn days_until_expiry(&self, today: NaiveDate) -> i64 {
        (self.expires_at - today).num_days()
    }

    pub fn is_expired(&self, today: NaiveDate) -> bool {
        today > self.expires_at
    }
}