use std::str::FromStr;

use async_trait::async_trait;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::{
    entities::armory::{CatalogEntry, WeaponClass},
    errors::{DomainError, DomainResult},
    repositories::armory::{CatalogFilter, GunCatalogRepository},
};

pub struct PgGunCatalogRepository { pool: PgPool }

impl PgGunCatalogRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

fn row_to_entry(r: PgRow) -> DomainResult<CatalogEntry> {
    let class_s: String = r.get("class");
    let class = WeaponClass::from_str(&class_s)
        .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad catalog class: {e}")))?;
    Ok(CatalogEntry {
        id: r.get("id"),
        manufacturer: r.get("manufacturer"),
        model: r.get("model"),
        class,
        caliber: r.get("caliber"),
        barrel_length_mm: r.get("barrel_length_mm"),
        weight_g: r.get("weight_g"),
        capacity: r.get("capacity"),
        notes: r.get("notes"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}

fn unique_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505"))
}

#[async_trait]
impl GunCatalogRepository for PgGunCatalogRepository {
    async fn create(&self, e: &CatalogEntry) -> DomainResult<()> {
        let res = sqlx::query(
            r#"
            INSERT INTO gun_catalog
              (id, manufacturer, model, class, caliber,
               barrel_length_mm, weight_g, capacity, notes,
               created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
            "#,
        )
        .bind(e.id).bind(&e.manufacturer).bind(&e.model)
        .bind(e.class.as_str()).bind(&e.caliber)
        .bind(e.barrel_length_mm).bind(e.weight_g).bind(e.capacity)
        .bind(&e.notes).bind(e.created_at).bind(e.updated_at)
        .execute(&self.pool)
        .await;

        if let Err(err) = res {
            if unique_violation(&err) {

                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        "manufacturer+model already exists".into())));
            }
            return Err(DomainError::from(err));
        }
        Ok(())
    }

    async fn update(&self, e: &CatalogEntry) -> DomainResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE gun_catalog SET
              manufacturer = $1, model = $2, class = $3, caliber = $4,
              barrel_length_mm = $5, weight_g = $6, capacity = $7,
              notes = $8, updated_at = NOW()
            WHERE id = $9
            "#,
        )
        .bind(&e.manufacturer).bind(&e.model).bind(e.class.as_str())
        .bind(&e.caliber).bind(e.barrel_length_mm).bind(e.weight_g)
        .bind(e.capacity).bind(&e.notes).bind(e.id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::GunNotFound);
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        let result = sqlx::query("DELETE FROM gun_catalog WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::GunNotFound);
        }
        Ok(())
    }

    async fn find(&self, id: Uuid) -> DomainResult<Option<CatalogEntry>> {
        let row = sqlx::query(
            r#"
            SELECT id, manufacturer, model, class, caliber,
                   barrel_length_mm, weight_g, capacity, notes,
                   created_at, updated_at
            FROM gun_catalog WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_entry).transpose()
    }

    async fn list(
        &self,
        filter: &CatalogFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<CatalogEntry>, i64)> {
        let mut where_sql = String::from("WHERE TRUE");
        let mut idx: i32 = 1;
        if filter.class.is_some()   { where_sql.push_str(&format!(" AND class = ${idx}"));   idx += 1; }
        if filter.caliber.is_some() { where_sql.push_str(&format!(" AND caliber = ${idx}")); idx += 1; }
        if filter.q.is_some() {
            where_sql.push_str(&format!(
                " AND (manufacturer ILIKE ${idx} OR model ILIKE ${idx})"));
            idx += 1;
        }
        let _ = idx;

        let count_sql = format!("SELECT COUNT(*) AS c FROM gun_catalog {where_sql}");
        let mut cq = sqlx::query(&count_sql);
        if let Some(c) = filter.class   { cq = cq.bind(c.as_str()); }
        if let Some(cal) = &filter.caliber { cq = cq.bind(cal); }
        if let Some(q) = &filter.q { cq = cq.bind(format!("%{q}%")); }
        let total: i64 = cq.fetch_one(&self.pool).await?.get("c");

        let sel_sql = format!(
            "SELECT id, manufacturer, model, class, caliber, \
                    barrel_length_mm, weight_g, capacity, notes, \
                    created_at, updated_at \
             FROM gun_catalog {where_sql} \
             ORDER BY manufacturer ASC, model ASC \
             LIMIT ${limit_idx} OFFSET ${offset_idx}",
            limit_idx = idx,
            offset_idx = idx + 1,
        );
        let mut q = sqlx::query(&sel_sql);
        if let Some(c) = filter.class   { q = q.bind(c.as_str()); }
        if let Some(cal) = &filter.caliber { q = q.bind(cal); }
        if let Some(qq) = &filter.q { q = q.bind(format!("%{qq}%")); }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        let items = rows.into_iter().map(row_to_entry).collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }
}
