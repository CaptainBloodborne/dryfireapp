//! Postgres implementation of [`AmmoRepository`].
//!
//! Implementation notes:
//!
//! - `record_transaction` runs
//!   in a transaction that does (1) INSERT into the ledger, (2)
//!   UPSERT into `ammo_stocks` adding the delta. If the resulting
//!   stock would go negative, the CHECK constraint trips and the
//!   whole transaction rolls back — preventing impossible state from
//!   ever being persisted.
//! - `usage_over_time` uses `generate_series` + `LEFT JOIN` so we
//!   return a row for every bucket in the range, even empty ones.

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::{
    entities::ammo::{
        AmmoStock, AmmoTransaction, AmmoType, BulletType, ProjectileType,
    },
    errors::{DomainError, DomainResult},
    repositories::ammo::{
        AmmoRepository, AmmoTypeFilter, BucketSize, CaliberSummary,
        GunUsageSummary, StockFilter, StockWithType, TimeBucketUsage,
        TransactionDirection, TransactionFilter,
    },
};

pub struct PgAmmoRepository { pool: PgPool }

impl PgAmmoRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

fn unique_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505"))
}

/// Detects PostgreSQL CHECK-constraint violation (SQLSTATE 23514).
fn check_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.code().as_deref() == Some("23514"))
}

fn ammo_type_row(r: PgRow) -> DomainResult<AmmoType> {
    let bt_s: String = r.get("bullet_type");
    let pt_s: String = r.get("projectile_type");
    Ok(AmmoType {
        id: r.get("id"),
        manufacturer: r.get("manufacturer"),
        name: r.get("name"),
        caliber: r.get("caliber"),
        bullet_type: BulletType::from_str(&bt_s)
            .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad bullet_type: {e}")))?,
        projectile_type: ProjectileType::from_str(&pt_s)
            .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad projectile_type: {e}")))?,
        powder_charge_grain: r.get::<Option<f64>, _>("powder_charge_grain"),
        bullet_weight_grain: r.get::<Option<f64>, _>("bullet_weight_grain"),
        notes: r.get::<Option<String>, _>("notes"),
        created_at: r.get::<DateTime<Utc>, _>("created_at"),
        updated_at: r.get::<DateTime<Utc>, _>("updated_at"),
    })
}

fn transaction_row(r: PgRow) -> AmmoTransaction {
    AmmoTransaction {
        id: r.get("id"),
        user_id: r.get("user_id"),
        ammo_type_id: r.get("ammo_type_id"),
        gun_id: r.get::<Option<Uuid>, _>("gun_id"),
        delta: r.get("delta"),
        occurred_at: r.get("occurred_at"),
        note: r.get::<Option<String>, _>("note"),
        created_at: r.get("created_at"),
    }
}

#[async_trait]
impl AmmoRepository for PgAmmoRepository {
    // ammo types

    async fn type_create(&self, t: &AmmoType) -> DomainResult<()> {
        let res = sqlx::query(
            r#"
            INSERT INTO ammo_types
              (id, manufacturer, name, caliber, bullet_type, projectile_type,
               powder_charge_grain, bullet_weight_grain, notes,
               created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5::bullet_type,$6::projectile_type,
                    $7,$8,$9,$10,$11)
            "#,
        )
        .bind(t.id).bind(&t.manufacturer).bind(&t.name).bind(&t.caliber)
        .bind(t.bullet_type.as_str()).bind(t.projectile_type.as_str())
        .bind(t.powder_charge_grain).bind(t.bullet_weight_grain)
        .bind(&t.notes).bind(t.created_at).bind(t.updated_at)
        .execute(&self.pool)
        .await;

        if let Err(e) = res {
            if unique_violation(&e) {
                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        "ammo_type (manufacturer + name + caliber) already exists".into())));
            }
            return Err(DomainError::from(e));
        }
        Ok(())
    }

    async fn type_update(&self, t: &AmmoType) -> DomainResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE ammo_types SET
              manufacturer = $1, name = $2, caliber = $3,
              bullet_type = $4::bullet_type,
              projectile_type = $5::projectile_type,
              powder_charge_grain = $6, bullet_weight_grain = $7,
              notes = $8, updated_at = NOW()
            WHERE id = $9
            "#,
        )
        .bind(&t.manufacturer).bind(&t.name).bind(&t.caliber)
        .bind(t.bullet_type.as_str()).bind(t.projectile_type.as_str())
        .bind(t.powder_charge_grain).bind(t.bullet_weight_grain)
        .bind(&t.notes).bind(t.id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::AmmoLotNotFound);
        }
        Ok(())
    }

    async fn type_delete(&self, id: Uuid) -> DomainResult<()> {
        let result = sqlx::query("DELETE FROM ammo_types WHERE id = $1")
            .bind(id).execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::AmmoLotNotFound);
        }
        Ok(())
    }

    async fn type_find(&self, id: Uuid) -> DomainResult<Option<AmmoType>> {
        let row = sqlx::query(
            r#"
            SELECT id, manufacturer, name, caliber,
                   bullet_type::text AS bullet_type,
                   projectile_type::text AS projectile_type,
                   powder_charge_grain, bullet_weight_grain, notes,
                   created_at, updated_at
            FROM ammo_types WHERE id = $1
            "#,
        )
        .bind(id).fetch_optional(&self.pool).await?;
        row.map(ammo_type_row).transpose()
    }

    async fn type_list(
        &self,
        filter: &AmmoTypeFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<AmmoType>, i64)> {
        let mut where_sql = String::from("WHERE TRUE");
        let mut idx: i32 = 1;

        if filter.manufacturer.is_some() {
            where_sql.push_str(&format!(" AND manufacturer ILIKE ${idx}")); idx += 1;
        }
        if filter.caliber.is_some() {
            where_sql.push_str(&format!(" AND caliber = ${idx}")); idx += 1;
        }
        if filter.bullet_type.is_some() {
            where_sql.push_str(&format!(" AND bullet_type = ${idx}::bullet_type")); idx += 1;
        }
        if filter.projectile_type.is_some() {
            where_sql.push_str(&format!(" AND projectile_type = ${idx}::projectile_type")); idx += 1;
        }
        if filter.powder_charge_min.is_some() {
            where_sql.push_str(&format!(" AND powder_charge_grain >= ${idx}")); idx += 1;
        }
        if filter.powder_charge_max.is_some() {
            where_sql.push_str(&format!(" AND powder_charge_grain <= ${idx}")); idx += 1;
        }
        if filter.q.is_some() {
            where_sql.push_str(&format!(
                " AND (manufacturer ILIKE ${idx} OR name ILIKE ${idx} OR COALESCE(notes,'') ILIKE ${idx})"));
            idx += 1;
        }

        // ---- count ----
        let count_sql = format!("SELECT COUNT(*) AS c FROM ammo_types {where_sql}");
        let mut cq = sqlx::query(&count_sql);
        if let Some(m)  = &filter.manufacturer       { cq = cq.bind(format!("%{m}%")); }
        if let Some(c)  = &filter.caliber            { cq = cq.bind(c); }
        if let Some(b)  = filter.bullet_type         { cq = cq.bind(b.as_str()); }
        if let Some(p)  = filter.projectile_type     { cq = cq.bind(p.as_str()); }
        if let Some(p)  = filter.powder_charge_min   { cq = cq.bind(p); }
        if let Some(p)  = filter.powder_charge_max   { cq = cq.bind(p); }
        if let Some(q)  = &filter.q                  { cq = cq.bind(format!("%{q}%")); }
        let total: i64 = cq.fetch_one(&self.pool).await?.get("c");

        // ---- page ----
        let sel = format!(
            "SELECT id, manufacturer, name, caliber, \
                    bullet_type::text AS bullet_type, \
                    projectile_type::text AS projectile_type, \
                    powder_charge_grain, bullet_weight_grain, notes, \
                    created_at, updated_at \
             FROM ammo_types {where_sql} \
             ORDER BY manufacturer ASC, name ASC \
             LIMIT ${l_idx} OFFSET ${o_idx}",
            l_idx = idx, o_idx = idx + 1,
        );
        let mut q = sqlx::query(&sel);
        if let Some(m)  = &filter.manufacturer       { q = q.bind(format!("%{m}%")); }
        if let Some(c)  = &filter.caliber            { q = q.bind(c); }
        if let Some(b)  = filter.bullet_type         { q = q.bind(b.as_str()); }
        if let Some(p)  = filter.projectile_type     { q = q.bind(p.as_str()); }
        if let Some(p)  = filter.powder_charge_min   { q = q.bind(p); }
        if let Some(p)  = filter.powder_charge_max   { q = q.bind(p); }
        if let Some(qq) = &filter.q                  { q = q.bind(format!("%{qq}%")); }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        let items = rows.into_iter().map(ammo_type_row).collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }

    // transactions

    async fn record_transaction(
        &self,
        tx: &AmmoTransaction,
    ) -> DomainResult<AmmoStock> {
        let mut db_tx = self.pool.begin().await?;

        // If a gun_id is supplied, verify it belongs to the user.
        // Doing it as a single guarded INSERT or a precheck has the
        // same security guarantees; an explicit precheck gives a
        // nicer error message.
        if let Some(gid) = tx.gun_id {
            let owned: bool = sqlx::query(
                "SELECT EXISTS (SELECT 1 FROM guns WHERE id = $1 AND user_id = $2) AS o"
            )
            .bind(gid).bind(tx.user_id)
            .fetch_one(&mut *db_tx).await?
            .get("o");
            if !owned {
                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        format!("gun {gid} not found or not owned by user"))));
            }
        }

        // 1. Insert the ledger row.
        sqlx::query(
            r#"
            INSERT INTO ammo_transactions
              (id, user_id, ammo_type_id, gun_id, delta,
               occurred_at, note, created_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(tx.id).bind(tx.user_id).bind(tx.ammo_type_id)
        .bind(tx.gun_id).bind(tx.delta)
        .bind(tx.occurred_at).bind(&tx.note).bind(tx.created_at)
        .execute(&mut *db_tx)
        .await?;

        // 2. UPSERT the materialized stock row. On conflict, add the
        //    delta to the existing quantity. The CHECK (quantity >= 0)
        //    constraint will trip if a consumption would go below zero
        //    — we map that to a friendly typed error.
        let stock_res = sqlx::query(
            r#"
            INSERT INTO ammo_stocks (user_id, ammo_type_id, quantity, updated_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (user_id, ammo_type_id) DO UPDATE
              SET quantity   = ammo_stocks.quantity + EXCLUDED.quantity,
                  updated_at = NOW()
            RETURNING quantity, updated_at
            "#,
        )
        .bind(tx.user_id).bind(tx.ammo_type_id).bind(tx.delta)
        .fetch_one(&mut *db_tx)
        .await;

        let (qty, updated_at) = match stock_res {
            Ok(row) => (
                row.get::<i32, _>("quantity"),
                row.get::<DateTime<Utc>, _>("updated_at"),
            ),
            Err(e) if check_violation(&e) => {
                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        "insufficient stock — consumption would go below zero".into())));
            }
            Err(e) => return Err(DomainError::from(e)),
        };

        // Edge case: an acquisition into a fresh user+type pair will
        // INSERT a brand-new row with quantity == delta — fine. The
        // ON CONFLICT path only fires when a row already exists.

        db_tx.commit().await?;
        Ok(AmmoStock {
            user_id: tx.user_id,
            ammo_type_id: tx.ammo_type_id,
            quantity: qty,
            updated_at,
        })
    }

    async fn list_transactions(
        &self,
        user_id: Uuid,
        filter: &TransactionFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<AmmoTransaction>, i64)> {
        let mut where_sql = String::from("WHERE user_id = $1");
        let mut idx: i32 = 2;

        if filter.ammo_type_id.is_some() {
            where_sql.push_str(&format!(" AND ammo_type_id = ${idx}")); idx += 1;
        }
        if filter.gun_id.is_some() {
            where_sql.push_str(&format!(" AND gun_id = ${idx}")); idx += 1;
        }
        if filter.from.is_some() {
            where_sql.push_str(&format!(" AND occurred_at >= ${idx}")); idx += 1;
        }
        if filter.to.is_some() {
            where_sql.push_str(&format!(" AND occurred_at <= ${idx}")); idx += 1;
        }
        match filter.direction {
            Some(TransactionDirection::Acquire) => where_sql.push_str(" AND delta > 0"),
            Some(TransactionDirection::Consume) => where_sql.push_str(" AND delta < 0"),
            None => (),
        }

        // ---- count ----
        let count_sql = format!("SELECT COUNT(*) AS c FROM ammo_transactions {where_sql}");
        let mut cq = sqlx::query(&count_sql).bind(user_id);
        if let Some(a) = filter.ammo_type_id { cq = cq.bind(a); }
        if let Some(g) = filter.gun_id       { cq = cq.bind(g); }
        if let Some(f) = filter.from         { cq = cq.bind(f); }
        if let Some(t) = filter.to           { cq = cq.bind(t); }
        let total: i64 = cq.fetch_one(&self.pool).await?.get("c");

        // ---- page ----
        let sel = format!(
            "SELECT id, user_id, ammo_type_id, gun_id, delta, \
                    occurred_at, note, created_at \
             FROM ammo_transactions {where_sql} \
             ORDER BY occurred_at DESC \
             LIMIT ${l_idx} OFFSET ${o_idx}",
            l_idx = idx, o_idx = idx + 1,
        );
        let mut q = sqlx::query(&sel).bind(user_id);
        if let Some(a) = filter.ammo_type_id { q = q.bind(a); }
        if let Some(g) = filter.gun_id       { q = q.bind(g); }
        if let Some(f) = filter.from         { q = q.bind(f); }
        if let Some(t) = filter.to           { q = q.bind(t); }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        Ok((rows.into_iter().map(transaction_row).collect(), total))
    }

    // current stocks

    async fn list_stocks(
        &self,
        user_id: Uuid,
        filter: &StockFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<StockWithType>, i64)> {
        let mut where_sql = String::from("WHERE s.user_id = $1");
        let mut idx: i32 = 2;
        if !filter.include_zero {
            where_sql.push_str(" AND s.quantity > 0");
        }
        if filter.ammo_type_id.is_some() {
            where_sql.push_str(&format!(" AND s.ammo_type_id = ${idx}")); idx += 1;
        }
        if filter.caliber.is_some() {
            where_sql.push_str(&format!(" AND t.caliber = ${idx}")); idx += 1;
        }

        let count_sql = format!(
            "SELECT COUNT(*) AS c FROM ammo_stocks s \
             JOIN ammo_types t ON t.id = s.ammo_type_id {where_sql}",
        );
        let mut cq = sqlx::query(&count_sql).bind(user_id);
        if let Some(a) = filter.ammo_type_id { cq = cq.bind(a); }
        if let Some(c) = &filter.caliber     { cq = cq.bind(c); }
        let total: i64 = cq.fetch_one(&self.pool).await?.get("c");

        let sel = format!(
            "SELECT s.user_id, s.ammo_type_id, s.quantity, s.updated_at AS stock_updated_at, \
                    t.id, t.manufacturer, t.name, t.caliber, \
                    t.bullet_type::text AS bullet_type, \
                    t.projectile_type::text AS projectile_type, \
                    t.powder_charge_grain, t.bullet_weight_grain, t.notes, \
                    t.created_at, t.updated_at \
             FROM ammo_stocks s \
             JOIN ammo_types t ON t.id = s.ammo_type_id \
             {where_sql} \
             ORDER BY t.caliber ASC, t.manufacturer ASC, t.name ASC \
             LIMIT ${l_idx} OFFSET ${o_idx}",
            l_idx = idx, o_idx = idx + 1,
        );
        let mut q = sqlx::query(&sel).bind(user_id);
        if let Some(a) = filter.ammo_type_id { q = q.bind(a); }
        if let Some(c) = &filter.caliber     { q = q.bind(c); }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        let mut items: Vec<StockWithType> = Vec::with_capacity(rows.len());
        for r in rows {
            let stock = AmmoStock {
                user_id: r.get("user_id"),
                ammo_type_id: r.get("ammo_type_id"),
                quantity: r.get("quantity"),
                updated_at: r.get("stock_updated_at"),
            };
            let ammo_type = ammo_type_row(r)?;
            items.push(StockWithType { stock, ammo_type });
        }
        Ok((items, total))
    }

    // aggregations

    async fn summary_by_caliber(
        &self,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> DomainResult<Vec<CaliberSummary>> {
        let mut where_sql = String::from("WHERE tx.user_id = $1");
        let mut idx: i32 = 2;
        if from.is_some() { where_sql.push_str(&format!(" AND tx.occurred_at >= ${idx}")); idx += 1; }
        if to.is_some()   { where_sql.push_str(&format!(" AND tx.occurred_at <= ${idx}")); let _ = idx; }

        let sql = format!(
            "SELECT t.caliber, \
                    COALESCE(SUM(CASE WHEN tx.delta > 0 THEN tx.delta ELSE 0 END), 0)::BIGINT AS acquired, \
                    COALESCE(SUM(CASE WHEN tx.delta < 0 THEN -tx.delta ELSE 0 END), 0)::BIGINT AS consumed \
             FROM ammo_transactions tx \
             JOIN ammo_types t ON t.id = tx.ammo_type_id \
             {where_sql} \
             GROUP BY t.caliber \
             ORDER BY t.caliber ASC",
        );
        let mut q = sqlx::query(&sql).bind(user_id);
        if let Some(f) = from { q = q.bind(f); }
        if let Some(t) = to   { q = q.bind(t); }

        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| {
            let acquired: i64 = r.get("acquired");
            let consumed: i64 = r.get("consumed");
            CaliberSummary {
                caliber: r.get("caliber"),
                acquired,
                consumed,
                net: acquired - consumed,
            }
        }).collect())
    }

    async fn summary_by_gun(
        &self,
        user_id: Uuid,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> DomainResult<Vec<GunUsageSummary>> {
        let mut where_sql = String::from(
            "WHERE user_id = $1 AND gun_id IS NOT NULL AND delta < 0",
        );
        let mut idx: i32 = 2;
        if from.is_some() { where_sql.push_str(&format!(" AND occurred_at >= ${idx}")); idx += 1; }
        if to.is_some()   { where_sql.push_str(&format!(" AND occurred_at <= ${idx}")); let _ = idx; }

        let sql = format!(
            "SELECT gun_id, \
                    COALESCE(SUM(-delta), 0)::BIGINT AS rounds_fired, \
                    MAX(occurred_at) AS last_used_at \
             FROM ammo_transactions \
             {where_sql} \
             GROUP BY gun_id \
             ORDER BY rounds_fired DESC",
        );
        let mut q = sqlx::query(&sql).bind(user_id);
        if let Some(f) = from { q = q.bind(f); }
        if let Some(t) = to   { q = q.bind(t); }

        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| GunUsageSummary {
            gun_id: r.get::<Uuid, _>("gun_id"),
            rounds_fired: r.get("rounds_fired"),
            last_used_at: r.get::<Option<DateTime<Utc>>, _>("last_used_at"),
        }).collect())
    }

    async fn usage_over_time(
        &self,
        user_id: Uuid,
        bucket: BucketSize,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        ammo_type_id: Option<Uuid>,
        caliber: Option<&str>,
    ) -> DomainResult<Vec<TimeBucketUsage>> {
        // Bucket size comes from a validated enum — never interpolated
        // from user-supplied SQL. The bound parameters use indices:
        //   $1 = user_id
        //   $2 = from
        //   $3 = to
        //   $4 = ammo_type_id (when present)
        //   $5 = caliber (when present, possibly $4 if ammo_type_id absent)
        let bucket_str = bucket.as_sql();

        // Build the tx-side filters separately so the JOIN/WHERE
        // composition stays straightforward.
        let mut tx_filters = String::new();
        let mut idx: i32 = 4;
        if ammo_type_id.is_some() {
            tx_filters.push_str(&format!(" AND tx.ammo_type_id = ${idx}")); idx += 1;
        }
        if caliber.is_some() {
            tx_filters.push_str(&format!(" AND t.caliber = ${idx}")); idx += 1;
        }
        let _ = idx;

        // generate_series gives us one row per bucket in [from, to];
        // LEFT JOIN preserves empty buckets as zeros (better for charts
        // than gaps).
        let sql = format!(
            "WITH buckets AS ( \
                SELECT generate_series( \
                    date_trunc('{bucket_str}', $2::timestamptz), \
                    date_trunc('{bucket_str}', $3::timestamptz), \
                    interval '1 {bucket_str}' \
                ) AS bucket \
            ) \
            SELECT b.bucket::date AS bucket, \
                   COALESCE(SUM(CASE WHEN tx.delta > 0 THEN tx.delta ELSE 0 END), 0)::BIGINT AS acquired, \
                   COALESCE(SUM(CASE WHEN tx.delta < 0 THEN -tx.delta ELSE 0 END), 0)::BIGINT AS consumed \
            FROM buckets b \
            LEFT JOIN ammo_transactions tx \
                ON tx.user_id = $1 \
                AND tx.occurred_at >= $2 AND tx.occurred_at <= $3 \
                AND date_trunc('{bucket_str}', tx.occurred_at) = b.bucket \
            LEFT JOIN ammo_types t ON t.id = tx.ammo_type_id \
            WHERE TRUE {tx_filters} \
            GROUP BY b.bucket \
            ORDER BY b.bucket ASC",
        );

        let mut q = sqlx::query(&sql)
            .bind(user_id).bind(from).bind(to);
        if let Some(a) = ammo_type_id { q = q.bind(a); }
        if let Some(c) = caliber      { q = q.bind(c); }

        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| TimeBucketUsage {
            bucket: r.get::<NaiveDate, _>("bucket"),
            acquired: r.get("acquired"),
            consumed: r.get("consumed"),
        }).collect())
    }
}
