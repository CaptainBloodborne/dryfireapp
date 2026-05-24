// src/domain/repositories/armory.rs

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::{
    entities::armory::{AmmoLot, AmmoTransaction, AmmoTxnKind, Gun},
    errors::DomainResult,
};

#[derive(Debug, Clone, Default)]
pub struct PageQuery {
    pub page: u32,
    pub size: u32,
    pub sort: Option<String>,
    pub filter_text: Option<String>,
}
impl PageQuery {
    pub fn offset(&self) -> i64 { (self.page.saturating_sub(1) as i64) * self.size.max(1) as i64 }
    pub fn limit(&self) -> i64 { self.size.clamp(1, 200) as i64 }
}

#[async_trait]
pub trait GunRepository: Send + Sync {
    async fn create(&self, gun: &Gun, serial_cipher: &str, serial_hmac: &str)
        -> DomainResult<()>;
    async fn find_by_id(&self, id: Uuid, owner_id: Uuid) -> DomainResult<Option<Gun>>;
    async fn list_for_owner(&self, owner_id: Uuid, page: PageQuery)
        -> DomainResult<(Vec<Gun>, i64)>;
    async fn update(&self, gun: &Gun) -> DomainResult<()>;
    async fn soft_delete(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()>;
}

#[async_trait]
pub trait AmmoRepository: Send + Sync {
    async fn create_lot(&self, lot: &AmmoLot) -> DomainResult<()>;
    async fn find_lot(&self, id: Uuid, owner_id: Uuid) -> DomainResult<Option<AmmoLot>>;
    async fn list_lots(&self, owner_id: Uuid, page: PageQuery)
        -> DomainResult<(Vec<AmmoLot>, i64)>;
    async fn update_lot(&self, lot: &AmmoLot) -> DomainResult<()>;
    async fn delete_lot(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()>;

    /// Record a transaction *and* update the lot's denormalised count
    /// atomically. The repo MUST refuse negative deltas that would
    /// drive `quantity_on_hand` below zero (DomainError::AmmoInsufficient).
    async fn record_txn(&self, txn: &AmmoTransaction) -> DomainResult<()>;

    async fn list_txns(
        &self,
        owner_id: Uuid,
        gun_id: Option<Uuid>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        page: PageQuery,
    ) -> DomainResult<(Vec<AmmoTransaction>, i64)>;

    async fn usage_by_caliber(
        &self, owner_id: Uuid,
        from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>,
    ) -> DomainResult<Vec<(String, i64)>>;

    async fn usage_by_gun(
        &self, owner_id: Uuid,
        from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>,
    ) -> DomainResult<Vec<(Option<Uuid>, i64)>>;
}