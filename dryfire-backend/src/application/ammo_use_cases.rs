//! Ammo use cases.

use chrono::{DateTime, Utc};
use serde_json::json;
use uuid::Uuid;

use crate::{
    application::app_state::AppState,
    domain::{
        entities::ammo::{
            AmmoStock, AmmoTransaction, AmmoType, BulletType, ProjectileType,
        },
        errors::{DomainError, DomainResult},
        repositories::ammo::{
            AmmoTypeFilter, BucketSize, CaliberSummary, GunUsageSummary,
            StockFilter, StockWithType, TimeBucketUsage, TransactionFilter,
        },
        services::audit::AuditEntry,
    },
};

// =================== ammo type catalog (admin) =================== //

#[derive(Debug)]
pub struct CreateAmmoTypeInput {
    pub manufacturer: String,
    pub name: String,
    pub caliber: String,
    pub bullet_type: BulletType,
    pub projectile_type: ProjectileType,
    pub powder_charge_grain: Option<f64>,
    pub bullet_weight_grain: Option<f64>,
    pub notes: Option<String>,
}

pub struct CreateAmmoTypeUseCase<'a> { pub state: &'a AppState }
impl<'a> CreateAmmoTypeUseCase<'a> {
    pub async fn execute(&self, admin_id: Uuid, input: CreateAmmoTypeInput)
        -> DomainResult<AmmoType>
    {
        let t = AmmoType::new(
            input.manufacturer, input.name, input.caliber,
            input.bullet_type, input.projectile_type,
            input.powder_charge_grain, input.bullet_weight_grain,
            input.notes,
        )?;
        self.state.ammo_repo.type_create(&t).await?;
        self.state.audit.record(
            AuditEntry::new("ammo_type.create")
                .user(admin_id)
                .resource("ammo_type", t.id),
        ).await;
        Ok(t)
    }
}

#[derive(Debug, Default)]
pub struct UpdateAmmoTypeInput {
    pub manufacturer: Option<String>,
    pub name: Option<String>,
    pub caliber: Option<String>,
    pub bullet_type: Option<BulletType>,
    pub projectile_type: Option<ProjectileType>,
    pub powder_charge_grain: Option<Option<f64>>,
    pub bullet_weight_grain: Option<Option<f64>>,
    pub notes: Option<Option<String>>,
}

pub struct UpdateAmmoTypeUseCase<'a> { pub state: &'a AppState }
impl<'a> UpdateAmmoTypeUseCase<'a> {
    pub async fn execute(
        &self, admin_id: Uuid, id: Uuid, input: UpdateAmmoTypeInput,
    ) -> DomainResult<AmmoType> {
        let mut t = self.state.ammo_repo.type_find(id).await?
            .ok_or(DomainError::AmmoLotNotFound)?;
        if let Some(v) = input.manufacturer        { t.manufacturer = v; }
        if let Some(v) = input.name                { t.name = v; }
        if let Some(v) = input.caliber             { t.caliber = v; }
        if let Some(v) = input.bullet_type         { t.bullet_type = v; }
        if let Some(v) = input.projectile_type     { t.projectile_type = v; }
        if let Some(v) = input.powder_charge_grain { t.powder_charge_grain = v; }
        if let Some(v) = input.bullet_weight_grain { t.bullet_weight_grain = v; }
        if let Some(v) = input.notes               { t.notes = v; }
        t.updated_at = Utc::now();
        self.state.ammo_repo.type_update(&t).await?;
        self.state.audit.record(
            AuditEntry::new("ammo_type.update")
                .user(admin_id)
                .resource("ammo_type", id),
        ).await;
        Ok(t)
    }
}

pub struct DeleteAmmoTypeUseCase<'a> { pub state: &'a AppState }
impl<'a> DeleteAmmoTypeUseCase<'a> {
    pub async fn execute(&self, admin_id: Uuid, id: Uuid) -> DomainResult<()> {
        self.state.ammo_repo.type_delete(id).await?;
        self.state.audit.record(
            AuditEntry::new("ammo_type.delete")
                .user(admin_id)
                .resource("ammo_type", id),
        ).await;
        Ok(())
    }
}

pub struct GetAmmoTypeUseCase<'a> { pub state: &'a AppState }
impl<'a> GetAmmoTypeUseCase<'a> {
    pub async fn execute(&self, id: Uuid) -> DomainResult<AmmoType> {
        self.state.ammo_repo.type_find(id).await?
            .ok_or(DomainError::AmmoLotNotFound)
    }
}

pub struct ListAmmoTypesUseCase<'a> { pub state: &'a AppState }
impl<'a> ListAmmoTypesUseCase<'a> {
    pub async fn execute(
        &self, filter: &AmmoTypeFilter, limit: i64, offset: i64,
    ) -> DomainResult<(Vec<AmmoType>, i64)> {
        self.state.ammo_repo.type_list(filter, limit, offset).await
    }
}

// =================== transactions (user) =================== //

#[derive(Debug)]
pub struct RecordTransactionInput {
    pub user_id: Uuid,
    pub ammo_type_id: Uuid,
    pub gun_id: Option<Uuid>,
    pub delta: i32,
    pub occurred_at: DateTime<Utc>,
    pub note: Option<String>,
}

pub struct RecordTransactionUseCase<'a> { pub state: &'a AppState }
impl<'a> RecordTransactionUseCase<'a> {
    #[tracing::instrument(skip(self, input),
        fields(user_id = %input.user_id, delta = input.delta))]
    pub async fn execute(
        &self, input: RecordTransactionInput,
    ) -> DomainResult<(AmmoTransaction, AmmoStock)> {
        // Verify the ammo_type exists. We don't need to do this for the
        // repo to function (the FK would catch it), but a friendlier
        // typed error here is worth the extra round-trip.
        if self.state.ammo_repo.type_find(input.ammo_type_id).await?.is_none() {
            return Err(DomainError::AmmoLotNotFound);
        }

        let tx = AmmoTransaction::new(
            input.user_id, input.ammo_type_id, input.gun_id,
            input.delta, input.occurred_at, input.note,
        )?;
        let stock = self.state.ammo_repo.record_transaction(&tx).await?;

        self.state.audit.record(
            AuditEntry::new(if input.delta > 0 { "ammo.acquire" } else { "ammo.consume" })
                .user(input.user_id)
                .resource("ammo_transaction", tx.id)
                .metadata(json!({
                    "ammo_type_id": input.ammo_type_id,
                    "delta": input.delta,
                    "resulting_quantity": stock.quantity,
                    "gun_id": input.gun_id,
                })),
        ).await;
        Ok((tx, stock))
    }
}

pub struct ListTransactionsUseCase<'a> { pub state: &'a AppState }
impl<'a> ListTransactionsUseCase<'a> {
    pub async fn execute(
        &self, user_id: Uuid, filter: &TransactionFilter,
        limit: i64, offset: i64,
    ) -> DomainResult<(Vec<AmmoTransaction>, i64)> {
        self.state.ammo_repo.list_transactions(user_id, filter, limit, offset).await
    }
}

// =================== current stocks (user) =================== //

pub struct ListStocksUseCase<'a> { pub state: &'a AppState }
impl<'a> ListStocksUseCase<'a> {
    pub async fn execute(
        &self, user_id: Uuid, filter: &StockFilter,
        limit: i64, offset: i64,
    ) -> DomainResult<(Vec<StockWithType>, i64)> {
        self.state.ammo_repo.list_stocks(user_id, filter, limit, offset).await
    }
}

// =================== statistics (user) =================== //

pub struct SummaryByCaliberUseCase<'a> { pub state: &'a AppState }
impl<'a> SummaryByCaliberUseCase<'a> {
    pub async fn execute(
        &self, user_id: Uuid,
        from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>,
    ) -> DomainResult<Vec<CaliberSummary>> {
        if let (Some(f), Some(t)) = (from, to) {
            if t < f {
                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        "`to` must be on or after `from`".into())));
            }
        }
        self.state.ammo_repo.summary_by_caliber(user_id, from, to).await
    }
}

pub struct SummaryByGunUseCase<'a> { pub state: &'a AppState }
impl<'a> SummaryByGunUseCase<'a> {
    pub async fn execute(
        &self, user_id: Uuid,
        from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>,
    ) -> DomainResult<Vec<GunUsageSummary>> {
        self.state.ammo_repo.summary_by_gun(user_id, from, to).await
    }
}

pub struct UsageOverTimeUseCase<'a> { pub state: &'a AppState }
impl<'a> UsageOverTimeUseCase<'a> {
    #[allow(clippy::too_many_arguments)]
    pub async fn execute(
        &self, user_id: Uuid,
        bucket: BucketSize,
        from: DateTime<Utc>, to: DateTime<Utc>,
        ammo_type_id: Option<Uuid>,
        caliber: Option<&str>,
    ) -> DomainResult<Vec<TimeBucketUsage>> {
        if to < from {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "`to` must be on or after `from`".into())));
        }
        // Light guard against absurd ranges that would produce huge
        // result sets. Day buckets over 10 years = ~3650 rows, still
        // fine. Day buckets over 100 years = 36500 rows — reject.
        let span_days = (to - from).num_days();
        if matches!(bucket, BucketSize::Day) && span_days > 366 * 5 {
            return Err(DomainError::Validation(
                crate::domain::errors::ValidationError::Custom(
                    "day-bucket range too large; use week or month for >5 years".into())));
        }
        self.state.ammo_repo
            .usage_over_time(user_id, bucket, from, to, ammo_type_id, caliber)
            .await
    }
}
