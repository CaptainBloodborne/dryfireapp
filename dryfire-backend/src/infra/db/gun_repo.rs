// src/infra/db/gun_repo.rs

use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::{
    entities::armory::{Caliber, Gun, WeaponClass},
    errors::{DomainError, DomainResult},
    repositories::armory::{GunRepository, PageQuery},
};

pub struct PgGunRepository { pool: PgPool }
impl PgGunRepository { pub fn new(pool: PgPool) -> Self { Self { pool } } }

#[async_trait]
impl GunRepository for PgGunRepository {
    async fn create(&self, gun: &Gun, serial_cipher: &str, serial_hmac: &str)
        -> DomainResult<()>
    {
        let res = sqlx::query(
            r#"
            INSERT INTO guns
                (id, owner_id, manufacturer, model, serial_cipher, serial_hmac,
                 class, caliber, date_of_purchase, photo_url, notes,
                 created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7::weapon_class,$8,$9,$10,$11,$12,$13)
            "#,
        )
        .bind(gun.id())
        .bind(gun.owner_id())
        .bind(gun.manufacturer())
        .bind(gun.model())
        .bind(serial_cipher)
        .bind(serial_hmac)
        .bind(gun.class().as_str())
        .bind(gun.caliber().as_str())
        .bind(gun.date_of_purchase())
        .bind(gun.photo_url())
        .bind(gun.notes())
        .bind(gun.created_at())
        .bind(gun.updated_at())
        .execute(&self.pool)
        .await;
        if let Err(sqlx::Error::Database(ref db)) = res {
            if db.code().as_deref() == Some("23505") {
                return Err(DomainError::GunSerialAlreadyExists);
            }
        }
        res.map(|_| ()).map_err(DomainError::from)
    }

    async fn find_by_id(&self, id: Uuid, owner_id: Uuid) -> DomainResult<Option<Gun>> {
        let row = sqlx::query(
            r#"SELECT id, owner_id, manufacturer, model, serial_cipher,
                      class::text AS class, caliber, date_of_purchase,
                      photo_url, notes, created_at, updated_at
               FROM guns
               WHERE id = $1 AND owner_id = $2 AND deleted_at IS NULL"#,
        )
        .bind(id).bind(owner_id)
        .fetch_optional(&self.pool).await?;
        row.map(row_to_gun).transpose()
    }

    async fn list_for_owner(&self, owner_id: Uuid, page: PageQuery)
        -> DomainResult<(Vec<Gun>, i64)>
    {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM guns WHERE owner_id=$1 AND deleted_at IS NULL",
        )
        .bind(owner_id)
        .fetch_one(&self.pool).await?;

        let rows = sqlx::query(
            r#"SELECT id, owner_id, manufacturer, model, serial_cipher,
                      class::text AS class, caliber, date_of_purchase,
                      photo_url, notes, created_at, updated_at
               FROM guns
               WHERE owner_id=$1 AND deleted_at IS NULL
               ORDER BY created_at DESC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(owner_id).bind(page.limit()).bind(page.offset())
        .fetch_all(&self.pool).await?;
        let guns = rows.into_iter().map(row_to_gun).collect::<DomainResult<Vec<_>>>()?;
        Ok((guns, total))
    }

    async fn update(&self, gun: &Gun) -> DomainResult<()> {
        let res = sqlx::query(
            r#"UPDATE guns SET
                  manufacturer=$3, model=$4, class=$5::weapon_class,
                  caliber=$6, date_of_purchase=$7, photo_url=$8, notes=$9,
                  updated_at=NOW()
               WHERE id=$1 AND owner_id=$2 AND deleted_at IS NULL"#,
        )
        .bind(gun.id()).bind(gun.owner_id())
        .bind(gun.manufacturer()).bind(gun.model())
        .bind(gun.class().as_str()).bind(gun.caliber().as_str())
        .bind(gun.date_of_purchase()).bind(gun.photo_url()).bind(gun.notes())
        .execute(&self.pool).await?;
        if res.rows_affected() == 0 { return Err(DomainError::GunNotFound); }
        Ok(())
    }

    async fn soft_delete(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()> {
        let res = sqlx::query(
            "UPDATE guns SET deleted_at=NOW() WHERE id=$1 AND owner_id=$2 AND deleted_at IS NULL",
        )
        .bind(id).bind(owner_id)
        .execute(&self.pool).await?;
        if res.rows_affected() == 0 { return Err(DomainError::GunNotFound); }
        Ok(())
    }
}

fn row_to_gun(r: sqlx::postgres::PgRow) -> DomainResult<Gun> {
    let class: WeaponClass = r.try_get::<String, _>("class")?
        .parse().map_err(|e| DomainError::Infra(anyhow::anyhow!("bad class: {e}")))?;
    let caliber = Caliber::parse(r.try_get::<String, _>("caliber")?)
        .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad caliber: {e}")))?;
    // The plaintext serial is held only in memory by the use case after decryption.
    // For now we keep the ciphertext as the "serial" field — swap in real AEAD.
    let serial = base64_url::decode(&r.try_get::<String, _>("serial_cipher")?)
        .map(|b| String::from_utf8_lossy(&b).to_string())
        .unwrap_or_default();

    Ok(Gun::rehydrate(
        r.try_get("id")?, r.try_get("owner_id")?,
        r.try_get("manufacturer")?, r.try_get("model")?, serial,
        class, caliber, r.try_get("date_of_purchase")?,
        r.try_get("photo_url")?, r.try_get("notes")?,
        r.try_get("created_at")?, r.try_get("updated_at")?,
    ))
}