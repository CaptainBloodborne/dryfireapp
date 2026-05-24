use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::{
    entities::armory::{
        AmmoLot, AmmoTransaction, AmmoTxnKind, BulletType, Caliber, ShellType,
    },
    errors::{DomainError, DomainResult},
    repositories::armory::{AmmoRepository, PageQuery},
};

pub struct PgAmmoRepository {
    pool: PgPool,
}

impl PgAmmoRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl AmmoRepository for PgAmmoRepository {
    async fn create_lot(&self, lot: &AmmoLot) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO ammo_lots
                (id, owner_id, manufacturer, caliber, bullet_type, shell_type,
                 bullet_weight_grains, powder_charge_grains,
                 quantity_on_hand, notes, created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5::bullet_type,$6::shell_type,
                    $7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(lot.id)
        .bind(lot.owner_id)
        .bind(&lot.manufacturer)
        .bind(lot.caliber.as_str())
        .bind(lot.bullet_type.as_str())
        .bind(lot.shell_type.as_str())
        .bind(lot.bullet_weight_grains)
        .bind(lot.powder_charge_grains)
        .bind(lot.quantity_on_hand)
        .bind(&lot.notes)
        .bind(lot.created_at)
        .bind(lot.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_lot(&self, id: Uuid, owner_id: Uuid)
        -> DomainResult<Option<AmmoLot>>
    {
        let row = sqlx::query(
            r#"SELECT id, owner_id, manufacturer, caliber,
                      bullet_type::text AS bullet_type,
                      shell_type::text  AS shell_type,
                      bullet_weight_grains, powder_charge_grains,
                      quantity_on_hand, notes, created_at, updated_at
               FROM ammo_lots
               WHERE id = $1 AND owner_id = $2"#,
        )
        .bind(id).bind(owner_id)
        .fetch_optional(&self.pool).await?;
        row.map(row_to_lot).transpose()
    }

    async fn list_lots(
        &self,
        owner_id: Uuid,
        page: PageQuery,
    ) -> DomainResult<(Vec<AmmoLot>, i64)> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ammo_lots WHERE owner_id = $1",
        )
        .bind(owner_id)
        .fetch_one(&self.pool).await?;

        let rows = sqlx::query(
            r#"SELECT id, owner_id, manufacturer, caliber,
                      bullet_type::text AS bullet_type,
                      shell_type::text  AS shell_type,
                      bullet_weight_grains, powder_charge_grains,
                      quantity_on_hand, notes, created_at, updated_at
               FROM ammo_lots
               WHERE owner_id = $1
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(owner_id).bind(page.limit()).bind(page.offset())
        .fetch_all(&self.pool).await?;

        let lots = rows.into_iter()
            .map(row_to_lot)
            .collect::<DomainResult<Vec<_>>>()?;
        Ok((lots, total))
    }

    async fn update_lot(&self, lot: &AmmoLot) -> DomainResult<()> {
        let res = sqlx::query(
            r#"UPDATE ammo_lots SET
                   manufacturer=$3, caliber=$4,
                   bullet_type=$5::bullet_type, shell_type=$6::shell_type,
                   bullet_weight_grains=$7, powder_charge_grains=$8,
                   notes=$9, updated_at=NOW()
               WHERE id=$1 AND owner_id=$2"#,
        )
        .bind(lot.id).bind(lot.owner_id)
        .bind(&lot.manufacturer).bind(lot.caliber.as_str())
        .bind(lot.bullet_type.as_str()).bind(lot.shell_type.as_str())
        .bind(lot.bullet_weight_grains).bind(lot.powder_charge_grains)
        .bind(&lot.notes)
        .execute(&self.pool).await?;
        if res.rows_affected() == 0 { return Err(DomainError::AmmoLotNotFound); }
        Ok(())
    }

    async fn delete_lot(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()> {
        let res = sqlx::query(
            "DELETE FROM ammo_lots WHERE id=$1 AND owner_id=$2",
        )
        .bind(id).bind(owner_id)
        .execute(&self.pool).await?;
        if res.rows_affected() == 0 { return Err(DomainError::AmmoLotNotFound); }
        Ok(())
    }

    /// One transaction: lock the lot row (SELECT FOR UPDATE), validate
    /// the new quantity, update the denormalised count, then INSERT the
    /// txn record. All-or-nothing.
    async fn record_txn(&self, txn: &AmmoTransaction) -> DomainResult<()> {
        let mut tx = self.pool.begin().await?;

        let current: Option<i64> = sqlx::query_scalar(
            r#"SELECT quantity_on_hand
               FROM ammo_lots
               WHERE id = $1 AND owner_id = $2
               FOR UPDATE"#,
        )
        .bind(txn.lot_id).bind(txn.owner_id)
        .fetch_optional(&mut *tx).await?;

        let current = current.ok_or(DomainError::AmmoLotNotFound)?;
        let new_qty = current + txn.delta;
        if new_qty < 0 {
            return Err(DomainError::AmmoInsufficient {
                have: current,
                need: txn.delta.unsigned_abs() as i64,
            });
        }

        sqlx::query(
            "UPDATE ammo_lots SET quantity_on_hand=$1, updated_at=NOW() WHERE id=$2",
        )
        .bind(new_qty).bind(txn.lot_id)
        .execute(&mut *tx).await?;

        sqlx::query(
            r#"INSERT INTO ammo_transactions
                   (id, owner_id, lot_id, gun_id, kind, delta,
                    happened_at, notes, created_at)
               VALUES ($1,$2,$3,$4,$5::ammo_txn_kind,$6,$7,$8,NOW())"#,
        )
        .bind(txn.id).bind(txn.owner_id).bind(txn.lot_id).bind(txn.gun_id)
        .bind(txn.kind.as_str()).bind(txn.delta)
        .bind(txn.happened_at).bind(&txn.notes)
        .execute(&mut *tx).await?;

        tx.commit().await?;
        Ok(())
    }

    async fn list_txns(
        &self,
        owner_id: Uuid,
        gun_id: Option<Uuid>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        page: PageQuery,
    ) -> DomainResult<(Vec<AmmoTransaction>, i64)> {
        // We use Option<T> binds so a NULL bind disables that filter via the
        // `$N IS NULL OR ...` idiom — keeps the query plan simple.
        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM ammo_transactions
               WHERE owner_id = $1
                 AND ($2::uuid IS NULL OR gun_id = $2)
                 AND ($3::timestamptz IS NULL OR happened_at >= $3)
                 AND ($4::timestamptz IS NULL OR happened_at <= $4)"#,
        )
        .bind(owner_id).bind(gun_id).bind(from).bind(to)
        .fetch_one(&self.pool).await?;

        let rows = sqlx::query(
            r#"SELECT id, owner_id, lot_id, gun_id, kind::text AS kind,
                      delta, happened_at, notes, created_at
               FROM ammo_transactions
               WHERE owner_id = $1
                 AND ($2::uuid IS NULL OR gun_id = $2)
                 AND ($3::timestamptz IS NULL OR happened_at >= $3)
                 AND ($4::timestamptz IS NULL OR happened_at <= $4)
               ORDER BY happened_at DESC
               LIMIT $5 OFFSET $6"#,
        )
        .bind(owner_id).bind(gun_id).bind(from).bind(to)
        .bind(page.limit()).bind(page.offset())
        .fetch_all(&self.pool).await?;

        let txns = rows.into_iter()
            .map(row_to_txn)
            .collect::<DomainResult<Vec<_>>>()?;
        Ok((txns, total))
    }

    async fn usage_by_caliber(
        &self,
        owner_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> DomainResult<Vec<(String, i64)>> {
        // Only count consumption (use/loss). Sum the magnitude per caliber.
        let rows = sqlx::query(
            r#"SELECT l.caliber AS caliber, COALESCE(SUM(-t.delta), 0)::BIGINT AS used
               FROM ammo_transactions t
               JOIN ammo_lots l ON l.id = t.lot_id
               WHERE t.owner_id = $1
                 AND t.kind IN ('use','loss')
                 AND ($2::timestamptz IS NULL OR t.happened_at >= $2)
                 AND ($3::timestamptz IS NULL OR t.happened_at <= $3)
               GROUP BY l.caliber
               ORDER BY used DESC"#,
        )
        .bind(owner_id).bind(from).bind(to)
        .fetch_all(&self.pool).await?;

        Ok(rows.into_iter().map(|r| {
            (r.get::<String, _>("caliber"), r.get::<i64, _>("used"))
        }).collect())
    }

    async fn usage_by_gun(
        &self,
        owner_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> DomainResult<Vec<(Option<Uuid>, i64)>> {
        let rows = sqlx::query(
            r#"SELECT gun_id, COALESCE(SUM(-delta), 0)::BIGINT AS used
               FROM ammo_transactions
               WHERE owner_id = $1
                 AND kind IN ('use','loss')
                 AND ($2::timestamptz IS NULL OR happened_at >= $2)
                 AND ($3::timestamptz IS NULL OR happened_at <= $3)
               GROUP BY gun_id
               ORDER BY used DESC"#,
        )
        .bind(owner_id).bind(from).bind(to)
        .fetch_all(&self.pool).await?;

        Ok(rows.into_iter().map(|r| {
            (r.get::<Option<Uuid>, _>("gun_id"), r.get::<i64, _>("used"))
        }).collect())
    }
}

fn row_to_lot(r: PgRow) -> DomainResult<AmmoLot> {
    let caliber = Caliber::parse(r.try_get::<String, _>("caliber")?)
        .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad caliber: {e}")))?;
    let bullet_type: BulletType = r.try_get::<String, _>("bullet_type")?
        .parse().map_err(|e| DomainError::Infra(anyhow::anyhow!("bad bullet_type: {e}")))?;
    let shell_type: ShellType = r.try_get::<String, _>("shell_type")?
        .parse().map_err(|e| DomainError::Infra(anyhow::anyhow!("bad shell_type: {e}")))?;

    Ok(AmmoLot {
        id: r.try_get("id")?,
        owner_id: r.try_get("owner_id")?,
        manufacturer: r.try_get("manufacturer")?,
        caliber,
        bullet_type,
        shell_type,
        bullet_weight_grains: r.try_get("bullet_weight_grains")?,
        powder_charge_grains: r.try_get("powder_charge_grains")?,
        quantity_on_hand: r.try_get("quantity_on_hand")?,
        notes: r.try_get("notes")?,
        created_at: r.try_get("created_at")?,
        updated_at: r.try_get("updated_at")?,
    })
}

fn row_to_txn(r: PgRow) -> DomainResult<AmmoTransaction> {
    let kind: AmmoTxnKind = r.try_get::<String, _>("kind")?
        .parse().map_err(|e| DomainError::Infra(anyhow::anyhow!("bad txn kind: {e}")))?;
    Ok(AmmoTransaction {
        id: r.try_get("id")?,
        owner_id: r.try_get("owner_id")?,
        lot_id: r.try_get("lot_id")?,
        gun_id: r.try_get("gun_id")?,
        kind,
        delta: r.try_get("delta")?,
        happened_at: r.try_get("happened_at")?,
        notes: r.try_get("notes")?,
        created_at: r.try_get("created_at")?,
    })
}