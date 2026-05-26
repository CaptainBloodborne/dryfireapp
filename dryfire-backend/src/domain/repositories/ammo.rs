//! Repository contracts for the ammo domain.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::ammo::{AmmoStock, AmmoTransaction, AmmoType, BulletType, ProjectileType},
    errors::DomainResult,
};

#[derive(Debug, Clone, Default)]
pub struct AmmoTypeFilter {
    pub manufacturer: Option<String>,
    pub caliber: Option<String>,
    pub bullet_type: Option<BulletType>,
    pub projectile_type: Option<ProjectileType>,
    /// Inclusive bound on powder charge in grains.
    pub powder_charge_min: Option<f64>,
    pub powder_charge_max: Option<f64>,
    /// Substring across manufacturer + name + notes.
    pub q: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct StockFilter {
    pub ammo_type_id: Option<Uuid>,
    pub caliber: Option<String>,
    pub include_zero: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TransactionFilter {
    pub ammo_type_id: Option<Uuid>,
    pub gun_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    /// `true` - only acquisitions (delta > 0), `false` - only consumption.
    pub direction: Option<TransactionDirection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionDirection { Acquire, Consume }

/// Stock + the joined ammo-type info, for "list my inventory" screens.
#[derive(Debug, Clone)]
pub struct StockWithType {
    pub stock: AmmoStock,
    pub ammo_type: AmmoType,
}

/// One row of the "totals by caliber" aggregation.
#[derive(Debug, Clone)]
pub struct CaliberSummary {
    pub caliber: String,
    /// Sum of positive deltas (acquired).
    pub acquired: i64,
    /// Sum of `|negative deltas|` (consumed).
    pub consumed: i64,
    /// `acquired - consumed`.
    pub net: i64,
}

/// One row of the "totals by gun" aggregation.
#[derive(Debug, Clone)]
pub struct GunUsageSummary {
    pub gun_id: Uuid,
    /// Only consumption transactions are counted here. Acquiring
    /// rounds isn't associated with a specific gun.
    pub rounds_fired: i64,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Time-bucketed usage row.
#[derive(Debug, Clone)]
pub struct TimeBucketUsage {
    /// Start of the bucket as a calendar date.
    pub bucket: NaiveDate,
    pub acquired: i64,
    pub consumed: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketSize { Day, Week, Month }

impl BucketSize {
    pub fn as_sql(&self) -> &'static str {
        match self {
            BucketSize::Day => "day",
            BucketSize::Week => "week",
            BucketSize::Month => "month",
        }
    }
}

#[async_trait]
pub trait AmmoRepository: Send + Sync {
    // ammo types (admin-managed catalog)
    async fn type_create(&self, t: &AmmoType) -> DomainResult<()>;
    async fn type_update(&self, t: &AmmoType) -> DomainResult<()>;
    async fn type_delete(&self, id: Uuid) -> DomainResult<()>;
    async fn type_find(&self, id: Uuid) -> DomainResult<Option<AmmoType>>;
    async fn type_list(
        &self,
        filter: &AmmoTypeFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<AmmoType>, i64)>;

    // transactions
    /// Insert a transaction *and* atomically update the user's stock
    /// row. Both writes happen in one DB transaction so the cached
    /// stock can never drift from the ledger.
    async fn record_transaction(
        &self,
        tx: &AmmoTransaction,
    ) -> DomainResult<AmmoStock>;

    async fn list_transactions(
        &self,
        user_id: Uuid,
        filter: &TransactionFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<AmmoTransaction>, i64)>;

    // current stock
    async fn list_stocks(
        &self,
        user_id: Uuid,
        filter: &StockFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<StockWithType>, i64)>;

    // aggregations / statistics
    async fn summary_by_caliber(
        &self,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> DomainResult<Vec<CaliberSummary>>;

    async fn summary_by_gun(
        &self,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> DomainResult<Vec<GunUsageSummary>>;

    async fn usage_over_time(
        &self,
        user_id: Uuid,
        bucket: BucketSize,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        ammo_type_id: Option<Uuid>,
        caliber: Option<&str>,
    ) -> DomainResult<Vec<TimeBucketUsage>>;
}
