use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{
    entities::ammo::{
        AmmoStock, AmmoTransaction, AmmoType, BulletType, ProjectileType,
    },
    repositories::ammo::{
        BucketSize, CaliberSummary, GunUsageSummary, StockWithType,
        TimeBucketUsage, TransactionDirection,
    },
};

// AmmoType DTOs (admin + read)

#[derive(Debug, Deserialize)]
pub struct CreateAmmoTypeRequest {
    pub manufacturer: String,
    pub name: String,
    pub caliber: String,
    pub bullet_type: BulletType,
    pub projectile_type: ProjectileType,
    pub powder_charge_grain: Option<f64>,
    pub bullet_weight_grain: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateAmmoTypeRequest {
    #[serde(default)] pub manufacturer: Option<String>,
    #[serde(default)] pub name: Option<String>,
    #[serde(default)] pub caliber: Option<String>,
    #[serde(default)] pub bullet_type: Option<BulletType>,
    #[serde(default)] pub projectile_type: Option<ProjectileType>,
    #[serde(default)] pub powder_charge_grain: Option<Option<f64>>,
    #[serde(default)] pub bullet_weight_grain: Option<Option<f64>>,
    #[serde(default)] pub notes: Option<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct AmmoTypeResponse {
    #[serde(flatten)]
    pub entry: AmmoType,
}

#[derive(Debug, Deserialize)]
pub struct AmmoTypeListQuery {
    pub manufacturer: Option<String>,
    pub caliber: Option<String>,
    pub bullet_type: Option<BulletType>,
    pub projectile_type: Option<ProjectileType>,
    pub powder_charge_min: Option<f64>,
    pub powder_charge_max: Option<f64>,
    pub q: Option<String>,
}

// Transactions

#[derive(Debug, Deserialize)]
pub struct RecordTransactionRequest {
    pub ammo_type_id: Uuid,
    pub gun_id: Option<Uuid>,
    /// Positive = acquire, negative = consume.
    pub delta: i32,
    pub occurred_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TransactionResponse {
    pub transaction: AmmoTransaction,
    pub resulting_stock: AmmoStockResponse,
}

#[derive(Debug, Serialize)]
pub struct AmmoStockResponse {
    pub user_id: Uuid,
    pub ammo_type_id: Uuid,
    pub quantity: i32,
    pub updated_at: DateTime<Utc>,
}
impl From<AmmoStock> for AmmoStockResponse {
    fn from(s: AmmoStock) -> Self {
        Self { user_id: s.user_id, ammo_type_id: s.ammo_type_id,
               quantity: s.quantity, updated_at: s.updated_at }
    }
}

#[derive(Debug, Deserialize)]
pub struct TransactionListQuery {
    pub ammo_type_id: Option<Uuid>,
    pub gun_id: Option<Uuid>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub direction: Option<DirectionParam>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DirectionParam { Acquire, Consume }

impl From<DirectionParam> for TransactionDirection {
    fn from(d: DirectionParam) -> Self {
        match d {
            DirectionParam::Acquire => TransactionDirection::Acquire,
            DirectionParam::Consume => TransactionDirection::Consume,
        }
    }
}

// Stocks

#[derive(Debug, Deserialize)]
pub struct StockListQuery {
    pub ammo_type_id: Option<Uuid>,
    pub caliber: Option<String>,
    /// Include zero-quantity rows. Default `false` — most users want
    /// only "what's on hand".
    #[serde(default)]
    pub include_zero: bool,
}

#[derive(Debug, Serialize)]
pub struct StockWithTypeResponse {
    pub user_id: Uuid,
    pub ammo_type: AmmoType,
    pub quantity: i32,
    pub updated_at: DateTime<Utc>,
}
impl From<StockWithType> for StockWithTypeResponse {
    fn from(s: StockWithType) -> Self {
        Self {
            user_id: s.stock.user_id,
            ammo_type: s.ammo_type,
            quantity: s.stock.quantity,
            updated_at: s.stock.updated_at,
        }
    }
}

// Aggregations

#[derive(Debug, Deserialize)]
pub struct RangeQuery {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct CaliberSummaryResponse {
    pub caliber: String,
    pub acquired: i64,
    pub consumed: i64,
    pub net: i64,
}
impl From<CaliberSummary> for CaliberSummaryResponse {
    fn from(s: CaliberSummary) -> Self {
        Self { caliber: s.caliber, acquired: s.acquired, consumed: s.consumed, net: s.net }
    }
}

#[derive(Debug, Serialize)]
pub struct GunUsageResponse {
    pub gun_id: Uuid,
    pub rounds_fired: i64,
    pub last_used_at: Option<DateTime<Utc>>,
}
impl From<GunUsageSummary> for GunUsageResponse {
    fn from(s: GunUsageSummary) -> Self {
        Self { gun_id: s.gun_id, rounds_fired: s.rounds_fired, last_used_at: s.last_used_at }
    }
}

#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    pub bucket: BucketParam,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub ammo_type_id: Option<Uuid>,
    pub caliber: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BucketParam { Day, Week, Month }

impl From<BucketParam> for BucketSize {
    fn from(b: BucketParam) -> Self {
        match b {
            BucketParam::Day   => BucketSize::Day,
            BucketParam::Week  => BucketSize::Week,
            BucketParam::Month => BucketSize::Month,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UsageBucketResponse {
    pub bucket: NaiveDate,
    pub acquired: i64,
    pub consumed: i64,
}
impl From<TimeBucketUsage> for UsageBucketResponse {
    fn from(b: TimeBucketUsage) -> Self {
        Self { bucket: b.bucket, acquired: b.acquired, consumed: b.consumed }
    }
}
