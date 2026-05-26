//! Repository contracts for the licenses domain.
//!
//! Two repos:
//! - [`LicenseRepository`] — user-owned licenses, type catalog,
//!   linked-guns junction table.
//! - [`LicenseNotificationRepository`] — bookkeeping for the
//!   reminder scheduler.

use async_trait::async_trait;
use chrono::NaiveDate;
use uuid::Uuid;

use crate::domain::{
    entities::license::{License, LicenseType},
    errors::DomainResult,
};

#[derive(Debug, Clone, Default)]
pub struct LicenseFilter {
    /// Restrict to licenses linked to this specific gun.
    pub gun_id: Option<Uuid>,
    /// Restrict to licenses of this type.
    pub type_id: Option<Uuid>,
    /// `true` - only expired; `false` - only valid; `None` - both.
    pub expired: Option<bool>,
    /// Free-text search across number / issuer / notes.
    pub q: Option<String>,
}

/// Pair returned by "deadlines in range" query.
#[derive(Debug, Clone)]
pub struct LicenseDeadline {
    pub license_id: Uuid,
    pub license_number: String,
    pub issuer: String,
    pub expires_at: NaiveDate,
}

/// Returned by the reminder scheduler — one row per license that is
/// about to expire AND for which the given `days_before` reminder
/// has not yet been sent.
#[derive(Debug, Clone)]
pub struct LicenseDueForReminder {
    pub license_id: Uuid,
    pub user_id: Uuid,
    pub license_number: String,
    pub expires_at: NaiveDate,
    pub days_before: i32,
}

#[async_trait]
pub trait LicenseRepository: Send + Sync {
    // license types (admin-managed reference)
    async fn type_create(&self, t: &LicenseType) -> DomainResult<()>;
    async fn type_update(&self, t: &LicenseType) -> DomainResult<()>;
    async fn type_delete(&self, id: Uuid) -> DomainResult<()>;
    async fn type_find(&self, id: Uuid) -> DomainResult<Option<LicenseType>>;
    async fn type_list(
        &self,
        region: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<LicenseType>, i64)>;

    // per-user licenses
    async fn create(&self, license: &License) -> DomainResult<()>;
    async fn update(&self, license: &License) -> DomainResult<()>;
    async fn delete(&self, user_id: Uuid, id: Uuid) -> DomainResult<()>;
    async fn find(&self, user_id: Uuid, id: Uuid) -> DomainResult<Option<License>>;
    async fn list_for_user(
        &self,
        user_id: Uuid,
        filter: &LicenseFilter,
        today: NaiveDate,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<License>, i64)>;

    /// Deadlines (expires_at) in `[from, to]` for the given user.
    /// Always sorted by `expires_at ASC`.
    async fn deadlines_in_range(
        &self,
        user_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> DomainResult<Vec<LicenseDeadline>>;

    /// Used by the scheduler: every (license, days_before) pair where
    /// the license is exactly `days_before` days from expiring and
    /// no notification for that pair has been recorded yet.
    async fn licenses_due_for_reminder(
        &self,
        days_thresholds: &[i32],
        today: NaiveDate,
    ) -> DomainResult<Vec<LicenseDueForReminder>>;
}

#[async_trait]
pub trait LicenseNotificationRepository: Send + Sync {
    /// Idempotently mark a reminder as sent. The unique constraint on
    /// (license_id, days_before) ensures duplicate calls become no-ops
    /// without raising an error.
    async fn mark_sent(&self, license_id: Uuid, days_before: i32) -> DomainResult<()>;
}
