// src/infra/db/ballistic_repo.rs

use async_trait::async_trait;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::{
    entities::ballistics::{BallisticInput, BallisticProfile},
    errors::{DomainError, DomainResult},
    repositories::{armory::PageQuery, ballistics::BallisticProfileRepository},
};

pub struct PgBallisticProfileRepository {
    pool: PgPool,
}

impl PgBallisticProfileRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl BallisticProfileRepository for PgBallisticProfileRepository {
    async fn create(&self, p: &BallisticProfile) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO ballistic_profiles
                (id, owner_id, name, gun_id, lot_id,
                 caliber, bullet_weight_grains, muzzle_velocity_mps,
                 ballistic_coefficient, sight_height_mm, zero_distance_m,
                 created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,NOW(),NOW())
            "#,
        )
        .bind(p.id)
        .bind(p.owner_id)
        .bind(&p.name)
        .bind(p.gun_id)
        .bind(p.lot_id)
        .bind(&p.input.caliber)
        .bind(p.input.bullet_weight_grains)
        .bind(p.input.muzzle_velocity_mps)
        .bind(p.input.ballistic_coefficient)
        .bind(p.input.sight_height_mm)
        .bind(p.input.zero_distance_m)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid, owner_id: Uuid)
        -> DomainResult<Option<BallisticProfile>>
    {
        let row = sqlx::query(
            r#"SELECT id, owner_id, name, gun_id, lot_id,
                      caliber, bullet_weight_grains, muzzle_velocity_mps,
                      ballistic_coefficient, sight_height_mm, zero_distance_m
               FROM ballistic_profiles
               WHERE id = $1 AND owner_id = $2"#,
        )
        .bind(id).bind(owner_id)
        .fetch_optional(&self.pool).await?;
        row.map(row_to_profile).transpose()
    }

    async fn list(
        &self,
        owner_id: Uuid,
        page: PageQuery,
    ) -> DomainResult<(Vec<BallisticProfile>, i64)> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM ballistic_profiles WHERE owner_id = $1",
        )
        .bind(owner_id).fetch_one(&self.pool).await?;

        let rows = sqlx::query(
            r#"SELECT id, owner_id, name, gun_id, lot_id,
                      caliber, bullet_weight_grains, muzzle_velocity_mps,
                      ballistic_coefficient, sight_height_mm, zero_distance_m
               FROM ballistic_profiles
               WHERE owner_id = $1
               ORDER BY name ASC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(owner_id).bind(page.limit()).bind(page.offset())
        .fetch_all(&self.pool).await?;

        let items = rows.into_iter()
            .map(row_to_profile)
            .collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }

    async fn update(&self, p: &BallisticProfile) -> DomainResult<()> {
        let res = sqlx::query(
            r#"UPDATE ballistic_profiles SET
                   name=$3, gun_id=$4, lot_id=$5,
                   caliber=$6, bullet_weight_grains=$7,
                   muzzle_velocity_mps=$8, ballistic_coefficient=$9,
                   sight_height_mm=$10, zero_distance_m=$11,
                   updated_at=NOW()
               WHERE id=$1 AND owner_id=$2"#,
        )
        .bind(p.id).bind(p.owner_id)
        .bind(&p.name).bind(p.gun_id).bind(p.lot_id)
        .bind(&p.input.caliber).bind(p.input.bullet_weight_grains)
        .bind(p.input.muzzle_velocity_mps).bind(p.input.ballistic_coefficient)
        .bind(p.input.sight_height_mm).bind(p.input.zero_distance_m)
        .execute(&self.pool).await?;
        if res.rows_affected() == 0 {
            return Err(DomainError::BallisticProfileNotFound);
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()> {
        let res = sqlx::query(
            "DELETE FROM ballistic_profiles WHERE id=$1 AND owner_id=$2",
        )
        .bind(id).bind(owner_id)
        .execute(&self.pool).await?;
        if res.rows_affected() == 0 {
            return Err(DomainError::BallisticProfileNotFound);
        }
        Ok(())
    }
}

fn row_to_profile(r: PgRow) -> DomainResult<BallisticProfile> {
    let input = BallisticInput {
        caliber: r.try_get("caliber")?,
        bullet_weight_grains: r.try_get("bullet_weight_grains")?,
        muzzle_velocity_mps: r.try_get("muzzle_velocity_mps")?,
        ballistic_coefficient: r.try_get("ballistic_coefficient")?,
        sight_height_mm: r.try_get("sight_height_mm")?,
        zero_distance_m: r.try_get("zero_distance_m")?,
    };
    Ok(BallisticProfile {
        id: r.try_get("id")?,
        owner_id: r.try_get("owner_id")?,
        name: r.try_get("name")?,
        gun_id: r.try_get("gun_id")?,
        lot_id: r.try_get("lot_id")?,
        input,
    })
}