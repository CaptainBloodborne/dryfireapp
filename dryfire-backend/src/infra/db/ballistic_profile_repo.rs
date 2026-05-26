use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as Json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::{
    entities::ballistics::BallisticProfile,
    errors::{DomainError, DomainResult},
    repositories::ballistics::BallisticProfileRepository,
    services::ballistics::{Atmosphere, Bullet, Sight},
};

pub struct PgBallisticProfileRepository { pool: PgPool }

impl PgBallisticProfileRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

fn row_to_domain(row: sqlx::postgres::PgRow) -> DomainResult<BallisticProfile> {
    let bullet: Json = row.get("bullet");
    let sight: Json = row.get("sight");
    let atmosphere: Json = row.get("atmosphere");

    Ok(BallisticProfile {
        id: row.get("id"),
        user_id: row.get("user_id"),
        name: row.get("name"),
        gun_id: row.get::<Option<Uuid>, _>("gun_id"),
        ammo_id: row.get::<Option<Uuid>, _>("ammo_id"),
        bullet: serde_json::from_value::<Bullet>(bullet)
            .map_err(|e| DomainError::Infra(e.into()))?,
        sight: serde_json::from_value::<Sight>(sight)
            .map_err(|e| DomainError::Infra(e.into()))?,
        default_atmosphere: serde_json::from_value::<Atmosphere>(atmosphere)
            .map_err(|e| DomainError::Infra(e.into()))?,
        created_at: row.get::<DateTime<Utc>, _>("created_at"),
        updated_at: row.get::<DateTime<Utc>, _>("updated_at"),
    })
}

#[async_trait]
impl BallisticProfileRepository for PgBallisticProfileRepository {
    async fn create(&self, p: &BallisticProfile) -> DomainResult<()> {
        let bullet = serde_json::to_value(&p.bullet).map_err(|e| DomainError::Infra(e.into()))?;
        let sight = serde_json::to_value(&p.sight).map_err(|e| DomainError::Infra(e.into()))?;
        let atmosphere = serde_json::to_value(&p.default_atmosphere)
            .map_err(|e| DomainError::Infra(e.into()))?;

        sqlx::query(
            r#"
            INSERT INTO ballistic_profiles
              (id, user_id, name, gun_id, ammo_id, bullet, sight, atmosphere, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(p.id).bind(p.user_id).bind(&p.name)
        .bind(p.gun_id).bind(p.ammo_id)
        .bind(bullet).bind(sight).bind(atmosphere)
        .bind(p.created_at).bind(p.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update(&self, p: &BallisticProfile) -> DomainResult<()> {
        let bullet = serde_json::to_value(&p.bullet).map_err(|e| DomainError::Infra(e.into()))?;
        let sight = serde_json::to_value(&p.sight).map_err(|e| DomainError::Infra(e.into()))?;
        let atmosphere = serde_json::to_value(&p.default_atmosphere)
            .map_err(|e| DomainError::Infra(e.into()))?;

        let result = sqlx::query(
            r#"
            UPDATE ballistic_profiles
            SET name = $1, gun_id = $2, ammo_id = $3,
                bullet = $4, sight = $5, atmosphere = $6,
                updated_at = NOW()
            WHERE id = $7 AND user_id = $8
            "#,
        )
        .bind(&p.name).bind(p.gun_id).bind(p.ammo_id)
        .bind(bullet).bind(sight).bind(atmosphere)
        .bind(p.id).bind(p.user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DomainError::BallisticProfileNotFound);
        }
        Ok(())
    }

    async fn find(&self, user_id: Uuid, id: Uuid) -> DomainResult<Option<BallisticProfile>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, name, gun_id, ammo_id, bullet, sight, atmosphere,
                   created_at, updated_at
            FROM ballistic_profiles
            WHERE user_id = $1 AND id = $2
            "#,
        )
        .bind(user_id).bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_domain).transpose()
    }

    async fn list_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<BallisticProfile>, i64)> {
        let total: i64 = sqlx::query(
            "SELECT COUNT(*) AS c FROM ballistic_profiles WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?
        .get("c");

        let rows = sqlx::query(
            r#"
            SELECT id, user_id, name, gun_id, ammo_id, bullet, sight, atmosphere,
                   created_at, updated_at
            FROM ballistic_profiles
            WHERE user_id = $1
            ORDER BY updated_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id).bind(limit).bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let items = rows.into_iter().map(row_to_domain).collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }

    async fn delete(&self, user_id: Uuid, id: Uuid) -> DomainResult<()> {
        sqlx::query("DELETE FROM ballistic_profiles WHERE user_id = $1 AND id = $2")
            .bind(user_id).bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
