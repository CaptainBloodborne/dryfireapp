//! Postgres implementation of [`LicenseRepository`].
//!
//! Notes:
//!
//! - `create`/`update` are wrapped in a single transaction because
//!   they touch both `licenses` and the `license_guns` junction.
//! - `gun_ids` is loaded via a subquery using `array_agg` so we get
//!   the whole license + its linked guns in one round-trip instead of
//!   an N+1 SELECT.
//! - `licenses_due_for_reminder` is the scheduler's hot query. We
//!   compute "days until expiry" in SQL and join against
//!   `license_notifications` with a LEFT JOIN / NULL filter to
//!   exclude already-sent reminders.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::{
    entities::license::{License, LicenseType},
    errors::{DomainError, DomainResult},
    repositories::license::{
        LicenseDeadline, LicenseDueForReminder, LicenseFilter, LicenseRepository,
    },
};

pub struct PgLicenseRepository { pool: PgPool }

impl PgLicenseRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

fn unique_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505"))
}

fn type_row(r: PgRow) -> DomainResult<LicenseType> {
    Ok(LicenseType {
        id: r.get("id"),
        code: r.get("code"),
        name: r.get("name"),
        region: r.get("region"),
        validity_days: r.get::<Option<i32>, _>("validity_days"),
        instructions: r.get::<Option<String>, _>("instructions"),
        created_at: r.get::<DateTime<Utc>, _>("created_at"),
        updated_at: r.get::<DateTime<Utc>, _>("updated_at"),
    })
}

fn license_row(r: PgRow) -> DomainResult<License> {
    // `gun_ids` comes from a subquery aggregate; may be NULL when the
    // license has no linked guns.
    let gun_ids: Option<Vec<Uuid>> = r.try_get("gun_ids").ok();
    Ok(License::rehydrate(
        r.get("id"),
        r.get("user_id"),
        r.get::<Option<Uuid>, _>("license_type_id"),
        r.get("license_number"),
        r.get("issuer"),
        r.get::<NaiveDate, _>("issued_at"),
        r.get::<NaiveDate, _>("expires_at"),
        r.get::<Option<String>, _>("notes"),
        r.get::<Option<String>, _>("scan_url"),
        gun_ids.unwrap_or_default(),
        r.get::<DateTime<Utc>, _>("created_at"),
        r.get::<DateTime<Utc>, _>("updated_at"),
    ))
}

const LICENSE_SELECT: &str = r#"
    SELECT
        l.id, l.user_id, l.license_type_id, l.license_number, l.issuer,
        l.issued_at, l.expires_at, l.notes, l.scan_url,
        l.created_at, l.updated_at,
        COALESCE(
            (SELECT array_agg(lg.gun_id) FROM license_guns lg WHERE lg.license_id = l.id),
            ARRAY[]::uuid[]
        ) AS gun_ids
    FROM licenses l
"#;

#[async_trait]
impl LicenseRepository for PgLicenseRepository {
    // types

    async fn type_create(&self, t: &LicenseType) -> DomainResult<()> {
        let res = sqlx::query(
            r#"
            INSERT INTO license_types
              (id, code, name, region, validity_days, instructions,
               created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
            "#,
        )
        .bind(t.id).bind(&t.code).bind(&t.name).bind(&t.region)
        .bind(t.validity_days).bind(&t.instructions)
        .bind(t.created_at).bind(t.updated_at)
        .execute(&self.pool)
        .await;

        if let Err(e) = res {
            if unique_violation(&e) {
                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        "license_type with this code already exists".into())));
            }
            return Err(DomainError::from(e));
        }
        Ok(())
    }

    async fn type_update(&self, t: &LicenseType) -> DomainResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE license_types SET
              code = $1, name = $2, region = $3,
              validity_days = $4, instructions = $5,
              updated_at = NOW()
            WHERE id = $6
            "#,
        )
        .bind(&t.code).bind(&t.name).bind(&t.region)
        .bind(t.validity_days).bind(&t.instructions)
        .bind(t.id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::LicenseNotFound);
        }
        Ok(())
    }

    async fn type_delete(&self, id: Uuid) -> DomainResult<()> {
        let result = sqlx::query("DELETE FROM license_types WHERE id = $1")
            .bind(id).execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::LicenseNotFound);
        }
        Ok(())
    }

    async fn type_find(&self, id: Uuid) -> DomainResult<Option<LicenseType>> {
        let r = sqlx::query(
            "SELECT id, code, name, region, validity_days, instructions, \
                    created_at, updated_at FROM license_types WHERE id = $1",
        )
        .bind(id).fetch_optional(&self.pool).await?;
        r.map(type_row).transpose()
    }

    async fn type_list(
        &self,
        region: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<LicenseType>, i64)> {
        let (count_sql, list_sql, has_region) = if region.is_some() {
            (
                "SELECT COUNT(*) AS c FROM license_types WHERE region = $1",
                "SELECT id, code, name, region, validity_days, instructions, \
                        created_at, updated_at FROM license_types \
                 WHERE region = $1 ORDER BY name ASC LIMIT $2 OFFSET $3",
                true,
            )
        } else {
            (
                "SELECT COUNT(*) AS c FROM license_types",
                "SELECT id, code, name, region, validity_days, instructions, \
                        created_at, updated_at FROM license_types \
                 ORDER BY region ASC, name ASC LIMIT $1 OFFSET $2",
                false,
            )
        };

        let total: i64 = if has_region {
            sqlx::query(count_sql).bind(region.unwrap())
                .fetch_one(&self.pool).await?.get("c")
        } else {
            sqlx::query(count_sql).fetch_one(&self.pool).await?.get("c")
        };

        let rows = if has_region {
            sqlx::query(list_sql).bind(region.unwrap()).bind(limit).bind(offset)
                .fetch_all(&self.pool).await?
        } else {
            sqlx::query(list_sql).bind(limit).bind(offset)
                .fetch_all(&self.pool).await?
        };

        let items = rows.into_iter().map(type_row).collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }

    // licenses

    async fn create(&self, license: &License) -> DomainResult<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO licenses
              (id, user_id, license_type_id, license_number, issuer,
               issued_at, expires_at, notes, scan_url,
               created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(license.id())
        .bind(license.user_id())
        .bind(license.license_type_id())
        .bind(license.license_number())
        .bind(license.issuer())
        .bind(license.issued_at())
        .bind(license.expires_at())
        .bind(license.notes())
        .bind(license.scan_url())
        .bind(license.created_at())
        .bind(license.updated_at())
        .execute(&mut *tx)
        .await?;

        for gid in license.gun_ids() {
            let inserted = sqlx::query(
                r#"
                INSERT INTO license_guns (license_id, gun_id)
                SELECT $1, $2
                WHERE EXISTS (
                    SELECT 1 FROM guns
                    WHERE id = $2 AND user_id = $3
                )
                "#,
            )
            .bind(license.id()).bind(gid).bind(license.user_id())
            .execute(&mut *tx)
            .await?;

            if inserted.rows_affected() == 0 {
                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        format!("gun {gid} not found or not owned by user"))));
            }
        }

        tx.commit().await?;
        Ok(())
    }

    async fn update(&self, license: &License) -> DomainResult<()> {
        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            r#"
            UPDATE licenses SET
              license_type_id = $1, license_number = $2, issuer = $3,
              issued_at = $4, expires_at = $5, notes = $6, scan_url = $7,
              updated_at = NOW()
            WHERE id = $8 AND user_id = $9
            "#,
        )
        .bind(license.license_type_id())
        .bind(license.license_number())
        .bind(license.issuer())
        .bind(license.issued_at())
        .bind(license.expires_at())
        .bind(license.notes())
        .bind(license.scan_url())
        .bind(license.id())
        .bind(license.user_id())
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::LicenseNotFound);
        }

        sqlx::query("DELETE FROM license_guns WHERE license_id = $1")
            .bind(license.id())
            .execute(&mut *tx)
            .await?;

        for gid in license.gun_ids() {
            let inserted = sqlx::query(
                r#"
                INSERT INTO license_guns (license_id, gun_id)
                SELECT $1, $2
                WHERE EXISTS (
                    SELECT 1 FROM guns
                    WHERE id = $2 AND user_id = $3
                )
                "#,
            )
            .bind(license.id()).bind(gid).bind(license.user_id())
            .execute(&mut *tx)
            .await?;
            if inserted.rows_affected() == 0 {
                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        format!("gun {gid} not found or not owned by user"))));
            }
        }

        sqlx::query("DELETE FROM license_notifications WHERE license_id = $1")
            .bind(license.id())
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn delete(&self, user_id: Uuid, id: Uuid) -> DomainResult<()> {
        let result = sqlx::query(
            "DELETE FROM licenses WHERE id = $1 AND user_id = $2",
        )
        .bind(id).bind(user_id).execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::LicenseNotFound);
        }
        Ok(())
    }

    async fn find(&self, user_id: Uuid, id: Uuid) -> DomainResult<Option<License>> {
        let sql = format!("{LICENSE_SELECT} WHERE l.user_id = $1 AND l.id = $2");
        let row = sqlx::query(&sql)
            .bind(user_id).bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(license_row).transpose()
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
        filter: &LicenseFilter,
        today: NaiveDate,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<License>, i64)> {
        // Build WHERE with bound params only.
        let mut where_sql = String::from("WHERE l.user_id = $1");
        let mut idx: i32 = 2;

        if filter.type_id.is_some() {
            where_sql.push_str(&format!(" AND l.license_type_id = ${idx}"));
            idx += 1;
        }
        if filter.gun_id.is_some() {
            where_sql.push_str(&format!(
                " AND EXISTS (SELECT 1 FROM license_guns lg \
                                WHERE lg.license_id = l.id AND lg.gun_id = ${idx})"));
            idx += 1;
        }
        if let Some(expired) = filter.expired {
            where_sql.push_str(&format!(
                " AND l.expires_at {} ${idx}",
                if expired { "<" } else { ">=" }));
            idx += 1;
        }
        if filter.q.is_some() {
            where_sql.push_str(&format!(
                " AND (l.license_number ILIKE ${idx} OR l.issuer ILIKE ${idx} \
                                                   OR COALESCE(l.notes,'') ILIKE ${idx})"));
            idx += 1;
        }

        // ---- count ----
        let count_sql = format!("SELECT COUNT(*) AS c FROM licenses l {where_sql}");
        let mut cq = sqlx::query(&count_sql).bind(user_id);
        if let Some(t) = filter.type_id { cq = cq.bind(t); }
        if let Some(g) = filter.gun_id  { cq = cq.bind(g); }
        if filter.expired.is_some()     { cq = cq.bind(today); }
        if let Some(q) = &filter.q      { cq = cq.bind(format!("%{q}%")); }
        let total: i64 = cq.fetch_one(&self.pool).await?.get("c");

        // ---- page ----
        let list_sql = format!(
            "{LICENSE_SELECT} {where_sql} \
             ORDER BY l.expires_at ASC \
             LIMIT ${limit_idx} OFFSET ${offset_idx}",
            limit_idx = idx,
            offset_idx = idx + 1,
        );
        let mut lq = sqlx::query(&list_sql).bind(user_id);
        if let Some(t) = filter.type_id { lq = lq.bind(t); }
        if let Some(g) = filter.gun_id  { lq = lq.bind(g); }
        if filter.expired.is_some()     { lq = lq.bind(today); }
        if let Some(q) = &filter.q      { lq = lq.bind(format!("%{q}%")); }
        lq = lq.bind(limit).bind(offset);

        let rows = lq.fetch_all(&self.pool).await?;
        let items = rows.into_iter().map(license_row).collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }

    async fn deadlines_in_range(
        &self,
        user_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> DomainResult<Vec<LicenseDeadline>> {
        let rows = sqlx::query(
            r#"
            SELECT id, license_number, issuer, expires_at
            FROM licenses
            WHERE user_id = $1 AND expires_at BETWEEN $2 AND $3
            ORDER BY expires_at ASC
            "#,
        )
        .bind(user_id).bind(from).bind(to)
        .fetch_all(&self.pool).await?;

        Ok(rows.into_iter().map(|r| LicenseDeadline {
            license_id: r.get("id"),
            license_number: r.get("license_number"),
            issuer: r.get("issuer"),
            expires_at: r.get("expires_at"),
        }).collect())
    }

    async fn licenses_due_for_reminder(
        &self,
        days_thresholds: &[i32],
        today: NaiveDate,
    ) -> DomainResult<Vec<LicenseDueForReminder>> {
        let rows = sqlx::query(
            r#"
            SELECT l.id AS license_id, l.user_id, l.license_number,
                   l.expires_at, t.days_before
            FROM licenses l
            CROSS JOIN unnest($1::int[]) AS t(days_before)
            LEFT JOIN license_notifications n
                ON n.license_id = l.id AND n.days_before = t.days_before
            WHERE n.id IS NULL
              AND (l.expires_at - $2) = t.days_before
            ORDER BY l.expires_at ASC
            "#,
        )
        .bind(days_thresholds)
        .bind(today)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| LicenseDueForReminder {
            license_id: r.get("license_id"),
            user_id: r.get("user_id"),
            license_number: r.get("license_number"),
            expires_at: r.get("expires_at"),
            days_before: r.get("days_before"),
        }).collect())
    }
}
