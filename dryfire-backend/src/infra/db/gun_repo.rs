//! Postgres implementation of the gun repository.
//!
//! The serial number is encrypted with AES-256-GCM at write and
//! decrypted at read. The cipher is injected via constructor so this
//! file knows nothing about key material or algorithm choice — that's
//! [`FieldCipher`]'s job.
//!
//! The plaintext serial never travels through the SQL prepared
//! statement, only its ciphertext does. Even if someone tcpdumps the
//! Postgres wire protocol or extracts the WAL, they get garbage.

use std::{str::FromStr, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use secrecy::{ExposeSecret, SecretString};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::{
    entities::armory::{Gun, WeaponClass},
    errors::{DomainError, DomainResult},
    repositories::armory::{GunFilter, GunRepository},
    services::cipher::FieldCipher,
};

pub struct PgGunRepository {
    pool: PgPool,
    cipher: Arc<dyn FieldCipher>,
}

impl PgGunRepository {
    pub fn new(pool: PgPool, cipher: Arc<dyn FieldCipher>) -> Self {
        Self { pool, cipher }
    }
}

/// Whitelist of client-sort-fields - SQL columns. Anything outside
/// this list is silently ignored at the use-case layer; see
/// `utils::paging::order_by_clause`.
fn validate_sort(field: &str) -> Option<&'static str> {
    match field {
        "created_at" => Some("created_at"),
        "updated_at" => Some("updated_at"),
        "purchase"   => Some("date_of_purchase"),
        "manufacturer" => Some("manufacturer"),
        "model"      => Some("model"),
        "caliber"    => Some("caliber"),
        _ => None,
    }
}

fn row_to_gun(row: PgRow, cipher: &dyn FieldCipher) -> DomainResult<Gun> {
    let class_s: String = row.get("class");
    let class = WeaponClass::from_str(&class_s)
        .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad weapon class in DB: {e}")))?;

    let serial_blob: Vec<u8> = row.get("serial_ciphertext");
    let plaintext_serial = cipher
        .decrypt_str(&serial_blob)
        .map_err(|e| DomainError::Infra(anyhow::anyhow!("serial decrypt failed: {e}")))?;

    Ok(Gun::rehydrate(
        row.get("id"),
        row.get("user_id"),
        row.get::<Option<Uuid>, _>("catalog_id"),
        row.get("manufacturer"),
        row.get("model"),
        class,
        row.get("caliber"),
        SecretString::from(plaintext_serial),
        row.get::<NaiveDate, _>("date_of_purchase"),
        row.get::<Option<String>, _>("photo_url"),
        row.get::<Option<String>, _>("notes"),
        row.get::<DateTime<Utc>, _>("created_at"),
        row.get::<DateTime<Utc>, _>("updated_at"),
    ))
}

#[async_trait]
impl GunRepository for PgGunRepository {
    async fn create(&self, gun: &Gun) -> DomainResult<()> {
        
        let ciphertext = self
            .cipher
            .encrypt_str(gun.serial().expose_secret())
            .map_err(|e| DomainError::Infra(anyhow::anyhow!("serial encrypt failed: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO guns
              (id, user_id, catalog_id, manufacturer, model, class, caliber,
               serial_ciphertext, serial_last4_plain, date_of_purchase,
               photo_url, notes, created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)
            "#,
        )
        .bind(gun.id())
        .bind(gun.user_id())
        .bind(gun.catalog_id())
        .bind(gun.manufacturer())
        .bind(gun.model())
        .bind(gun.class().as_str())
        .bind(gun.caliber())
        .bind(&ciphertext)
        .bind(gun.serial_last4())
        .bind(gun.date_of_purchase())
        .bind(gun.photo_url())
        .bind(gun.notes())
        .bind(gun.created_at())
        .bind(gun.updated_at())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update(&self, gun: &Gun) -> DomainResult<()> {
        
        // We always re-encrypt the serial because the cipher uses a
        // fresh nonce per call — no way to know if the serial changed
        // without comparing plaintext, and a re-encrypt is cheap.
        let ciphertext = self
            .cipher
            .encrypt_str(gun.serial().expose_secret())
            .map_err(|e| DomainError::Infra(anyhow::anyhow!("serial encrypt failed: {e}")))?;

        let result = sqlx::query(
            r#"
            UPDATE guns SET
              catalog_id = $1, manufacturer = $2, model = $3, class = $4,
              caliber = $5, serial_ciphertext = $6, serial_last4_plain = $7,
              date_of_purchase = $8, photo_url = $9, notes = $10,
              updated_at = NOW()
            WHERE id = $11 AND user_id = $12
            "#,
        )
        .bind(gun.catalog_id())
        .bind(gun.manufacturer())
        .bind(gun.model())
        .bind(gun.class().as_str())
        .bind(gun.caliber())
        .bind(&ciphertext)
        .bind(gun.serial_last4())
        .bind(gun.date_of_purchase())
        .bind(gun.photo_url())
        .bind(gun.notes())
        .bind(gun.id())
        .bind(gun.user_id())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DomainError::GunNotFound);
        }
        Ok(())
    }

    async fn delete(&self, user_id: Uuid, id: Uuid) -> DomainResult<()> {
        let result = sqlx::query(
            "DELETE FROM guns WHERE id = $1 AND user_id = $2",
        )
        .bind(id).bind(user_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::GunNotFound);
        }
        Ok(())
    }

    async fn find(&self, user_id: Uuid, id: Uuid) -> DomainResult<Option<Gun>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, catalog_id, manufacturer, model, class, caliber,
                   serial_ciphertext, date_of_purchase, photo_url, notes,
                   created_at, updated_at
            FROM guns
            WHERE user_id = $1 AND id = $2
            "#,
        )
        .bind(user_id).bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| row_to_gun(r, &*self.cipher)).transpose()
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
        filter: &GunFilter,
        limit: i64,
        offset: i64,
        sort: Option<&str>,
    ) -> DomainResult<(Vec<Gun>, i64)> {
        let mut where_sql = String::from("WHERE user_id = $1");
        let mut idx: i32 = 2;

        if filter.class.is_some()   { where_sql.push_str(&format!(" AND class = ${idx}"));   idx += 1; }
        if filter.caliber.is_some() { where_sql.push_str(&format!(" AND caliber = ${idx}")); idx += 1; }
        if filter.q.is_some() {
            // Substring match on a few columns.
            where_sql.push_str(&format!(
                " AND (manufacturer ILIKE ${idx} OR model ILIKE ${idx} OR COALESCE(notes,'') ILIKE ${idx})"));
            idx += 1;
        }
        let _ = idx;

        // ORDER BY column is validated via `validate_sort`.
        let order_col = sort
            .and_then(validate_sort)
            .unwrap_or("created_at");

        // count
        let count_sql = format!("SELECT COUNT(*) AS c FROM guns {where_sql}");
        let mut count_q = sqlx::query(&count_sql).bind(user_id);
        if let Some(c) = filter.class { count_q = count_q.bind(c.as_str()); }
        if let Some(cal) = &filter.caliber { count_q = count_q.bind(cal); }
        if let Some(q) = &filter.q { count_q = count_q.bind(format!("%{q}%")); }
        let total: i64 = count_q.fetch_one(&self.pool).await?.get("c");

        // page
        let select_sql = format!(
            "SELECT id, user_id, catalog_id, manufacturer, model, class, caliber, \
                    serial_ciphertext, date_of_purchase, photo_url, notes, \
                    created_at, updated_at \
             FROM guns {where_sql} \
             ORDER BY {order_col} DESC \
             LIMIT ${limit_idx} OFFSET ${offset_idx}",
            limit_idx = idx,
            offset_idx = idx + 1,
        );
        let mut q = sqlx::query(&select_sql).bind(user_id);
        if let Some(c) = filter.class { q = q.bind(c.as_str()); }
        if let Some(cal) = &filter.caliber { q = q.bind(cal); }
        if let Some(qq) = &filter.q { q = q.bind(format!("%{qq}%")); }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        let items = rows.into_iter()
            .map(|r| row_to_gun(r, &*self.cipher))
            .collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }
}
