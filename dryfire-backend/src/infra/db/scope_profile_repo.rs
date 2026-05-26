use async_trait::async_trait;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::{
    entities::scope::ScopeProfile,
    errors::{DomainError, DomainResult},
    repositories::scope::ScopeProfileRepository,
    services::scope::{AdjustmentUnit, ClickValue},
};

pub struct PgScopeProfileRepository { pool: PgPool }

impl PgScopeProfileRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

fn parse_unit(s: &str) -> DomainResult<AdjustmentUnit> {
    match s {
        "moa" => Ok(AdjustmentUnit::Moa),
        "iphy" => Ok(AdjustmentUnit::Iphy),
        "mil" => Ok(AdjustmentUnit::Mil),
        other => Err(DomainError::Infra(anyhow::anyhow!("bad scope unit {other}"))),
    }
}

fn unit_str(u: AdjustmentUnit) -> &'static str {
    match u {
        AdjustmentUnit::Moa => "moa",
        AdjustmentUnit::Iphy => "iphy",
        AdjustmentUnit::Mil => "mil",
    }
}

fn row_to_domain(row: sqlx::postgres::PgRow) -> DomainResult<ScopeProfile> {
    let unit_s: String = row.get("unit");
    let unit = parse_unit(&unit_s)?;
    let fraction: f64 = row.get("click_fraction");

    Ok(ScopeProfile {
        id: row.get("id"),
        user_id: row.get("user_id"),
        gun_id: row.get::<Option<Uuid>, _>("gun_id"),
        name: row.get("name"),
        unit,
        click_value: ClickValue { fraction_of_unit: fraction },
        max_elevation_units: row.get("max_elevation_units"),
        max_windage_units: row.get("max_windage_units"),
        mount_height_mm: row.get("mount_height_mm"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[async_trait]
impl ScopeProfileRepository for PgScopeProfileRepository {
    async fn create(&self, p: &ScopeProfile) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO scope_profiles
              (id, user_id, gun_id, name, unit, click_fraction,
               max_elevation_units, max_windage_units, mount_height_mm,
               created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(p.id).bind(p.user_id).bind(p.gun_id).bind(&p.name)
        .bind(unit_str(p.unit)).bind(p.click_value.fraction_of_unit)
        .bind(p.max_elevation_units).bind(p.max_windage_units)
        .bind(p.mount_height_mm).bind(p.created_at).bind(p.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update(&self, p: &ScopeProfile) -> DomainResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE scope_profiles
            SET gun_id = $1, name = $2, unit = $3, click_fraction = $4,
                max_elevation_units = $5, max_windage_units = $6,
                mount_height_mm = $7, updated_at = NOW()
            WHERE id = $8 AND user_id = $9
            "#,
        )
        .bind(p.gun_id).bind(&p.name).bind(unit_str(p.unit))
        .bind(p.click_value.fraction_of_unit)
        .bind(p.max_elevation_units).bind(p.max_windage_units)
        .bind(p.mount_height_mm).bind(p.id).bind(p.user_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::ScopeProfileNotFound);
        }
        Ok(())
    }

    async fn find(&self, user_id: Uuid, id: Uuid) -> DomainResult<Option<ScopeProfile>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, gun_id, name, unit, click_fraction,
                   max_elevation_units, max_windage_units, mount_height_mm,
                   created_at, updated_at
            FROM scope_profiles
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
    ) -> DomainResult<(Vec<ScopeProfile>, i64)> {
        let total: i64 = sqlx::query(
            "SELECT COUNT(*) AS c FROM scope_profiles WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?
        .get("c");

        let rows = sqlx::query(
            r#"
            SELECT id, user_id, gun_id, name, unit, click_fraction,
                   max_elevation_units, max_windage_units, mount_height_mm,
                   created_at, updated_at
            FROM scope_profiles
            WHERE user_id = $1
            ORDER BY name ASC
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
        sqlx::query("DELETE FROM scope_profiles WHERE user_id = $1 AND id = $2")
            .bind(user_id).bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
