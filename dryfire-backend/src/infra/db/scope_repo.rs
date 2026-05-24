// src/infra/db/scope_repo.rs

use async_trait::async_trait;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::{
    entities::{
        ballistics::AdjustmentUnit,
        scope::ScopeProfile,
    },
    errors::{DomainError, DomainResult},
    repositories::{armory::PageQuery, scope::ScopeProfileRepository},
};

pub struct PgScopeProfileRepository {
    pool: PgPool,
}

impl PgScopeProfileRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl ScopeProfileRepository for PgScopeProfileRepository {
    async fn create(&self, p: &ScopeProfile) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO scope_profiles
                (id, owner_id, gun_id, name, unit, click_value,
                 elevation_max_clicks, windage_max_clicks,
                 mount_height_mm, created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5::adjustment_unit,$6,$7,$8,$9,NOW(),NOW())
            "#,
        )
        .bind(p.id)
        .bind(p.owner_id)
        .bind(p.gun_id)
        .bind(&p.name)
        .bind(p.unit.as_str())
        .bind(p.click_value)
        .bind(p.elevation_max_clicks)
        .bind(p.windage_max_clicks)
        .bind(p.mount_height_mm)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid, owner_id: Uuid)
        -> DomainResult<Option<ScopeProfile>>
    {
        let row = sqlx::query(
            r#"SELECT id, owner_id, gun_id, name,
                      unit::text AS unit,
                      click_value, elevation_max_clicks,
                      windage_max_clicks, mount_height_mm
               FROM scope_profiles
               WHERE id = $1 AND owner_id = $2"#,
        )
        .bind(id).bind(owner_id)
        .fetch_optional(&self.pool).await?;
        row.map(row_to_scope).transpose()
    }

    async fn list(
        &self,
        owner_id: Uuid,
        page: PageQuery,
    ) -> DomainResult<(Vec<ScopeProfile>, i64)> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM scope_profiles WHERE owner_id = $1",
        )
        .bind(owner_id).fetch_one(&self.pool).await?;

        let rows = sqlx::query(
            r#"SELECT id, owner_id, gun_id, name,
                      unit::text AS unit,
                      click_value, elevation_max_clicks,
                      windage_max_clicks, mount_height_mm
               FROM scope_profiles
               WHERE owner_id = $1
               ORDER BY name ASC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(owner_id).bind(page.limit()).bind(page.offset())
        .fetch_all(&self.pool).await?;

        let items = rows.into_iter()
            .map(row_to_scope)
            .collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }

    async fn update(&self, p: &ScopeProfile) -> DomainResult<()> {
        let res = sqlx::query(
            r#"UPDATE scope_profiles SET
                   gun_id=$3, name=$4,
                   unit=$5::adjustment_unit, click_value=$6,
                   elevation_max_clicks=$7, windage_max_clicks=$8,
                   mount_height_mm=$9, updated_at=NOW()
               WHERE id=$1 AND owner_id=$2"#,
        )
        .bind(p.id).bind(p.owner_id)
        .bind(p.gun_id).bind(&p.name)
        .bind(p.unit.as_str()).bind(p.click_value)
        .bind(p.elevation_max_clicks).bind(p.windage_max_clicks)
        .bind(p.mount_height_mm)
        .execute(&self.pool).await?;
        if res.rows_affected() == 0 {
            return Err(DomainError::ScopeProfileNotFound);
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()> {
        let res = sqlx::query(
            "DELETE FROM scope_profiles WHERE id=$1 AND owner_id=$2",
        )
        .bind(id).bind(owner_id)
        .execute(&self.pool).await?;
        if res.rows_affected() == 0 {
            return Err(DomainError::ScopeProfileNotFound);
        }
        Ok(())
    }
}

fn row_to_scope(r: PgRow) -> DomainResult<ScopeProfile> {
    let unit: AdjustmentUnit = r.try_get::<String, _>("unit")?
        .parse()
        .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad unit: {e}")))?;
    Ok(ScopeProfile {
        id: r.try_get("id")?,
        owner_id: r.try_get("owner_id")?,
        gun_id: r.try_get("gun_id")?,
        name: r.try_get("name")?,
        unit,
        click_value: r.try_get("click_value")?,
        elevation_max_clicks: r.try_get("elevation_max_clicks")?,
        windage_max_clicks: r.try_get("windage_max_clicks")?,
        mount_height_mm: r.try_get("mount_height_mm")?,
    })
}