// src/infra/db/license_repo.rs

use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::{
    entities::license::{License, LicenseKind, LicenseStatus},
    errors::{DomainError, DomainResult},
    repositories::{armory::PageQuery, license::LicenseRepository},
};

pub struct PgLicenseRepository {
    pool: PgPool,
}

impl PgLicenseRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl LicenseRepository for PgLicenseRepository {
    async fn create(&self, lic: &License) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO licenses
                (id, owner_id, kind, issuing_org, issued_at, expires_at,
                 status, document_url, instructions, created_at, updated_at)
            VALUES ($1,$2,$3::license_kind,$4,$5,$6,$7::license_status,$8,$9,$10,$11)
            "#,
        )
        .bind(lic.id())
        .bind(lic.owner_id())
        .bind(lic.kind().as_str())
        .bind(lic.issuing_org())
        .bind(lic.issued_at())
        .bind(lic.expires_at())
        .bind(lic.status().as_str())
        .bind(lic.document_url())
        .bind(lic.instructions())
        .bind(lic.created_at())
        .bind(lic.updated_at())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid, owner_id: Uuid)
        -> DomainResult<Option<License>>
    {
        let row = sqlx::query(
            r#"SELECT id, owner_id,
                      kind::text   AS kind,
                      issuing_org, issued_at, expires_at,
                      status::text AS status,
                      document_url, instructions, created_at, updated_at
               FROM licenses
               WHERE id = $1 AND owner_id = $2"#,
        )
        .bind(id).bind(owner_id)
        .fetch_optional(&self.pool).await?;
        row.map(row_to_license).transpose()
    }

    async fn list(
        &self,
        owner_id: Uuid,
        page: PageQuery,
    ) -> DomainResult<(Vec<License>, i64)> {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM licenses WHERE owner_id = $1",
        )
        .bind(owner_id).fetch_one(&self.pool).await?;

        let rows = sqlx::query(
            r#"SELECT id, owner_id,
                      kind::text   AS kind,
                      issuing_org, issued_at, expires_at,
                      status::text AS status,
                      document_url, instructions, created_at, updated_at
               FROM licenses
               WHERE owner_id = $1
               ORDER BY expires_at ASC
               LIMIT $2 OFFSET $3"#,
        )
        .bind(owner_id).bind(page.limit()).bind(page.offset())
        .fetch_all(&self.pool).await?;

        let lics = rows.into_iter()
            .map(row_to_license)
            .collect::<DomainResult<Vec<_>>>()?;
        Ok((lics, total))
    }

    async fn update(&self, lic: &License) -> DomainResult<()> {
        let res = sqlx::query(
            r#"UPDATE licenses SET
                   kind=$3::license_kind, issuing_org=$4, issued_at=$5,
                   expires_at=$6, status=$7::license_status,
                   document_url=$8, instructions=$9, updated_at=NOW()
               WHERE id=$1 AND owner_id=$2"#,
        )
        .bind(lic.id()).bind(lic.owner_id())
        .bind(lic.kind().as_str()).bind(lic.issuing_org())
        .bind(lic.issued_at()).bind(lic.expires_at())
        .bind(lic.status().as_str())
        .bind(lic.document_url()).bind(lic.instructions())
        .execute(&self.pool).await?;
        if res.rows_affected() == 0 { return Err(DomainError::LicenseNotFound); }
        Ok(())
    }

    async fn delete(&self, id: Uuid, owner_id: Uuid) -> DomainResult<()> {
        let res = sqlx::query(
            "DELETE FROM licenses WHERE id = $1 AND owner_id = $2",
        )
        .bind(id).bind(owner_id)
        .execute(&self.pool).await?;
        if res.rows_affected() == 0 { return Err(DomainError::LicenseNotFound); }
        Ok(())
    }

    async fn deadlines_in(
        &self,
        owner_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> DomainResult<Vec<License>> {
        let rows = sqlx::query(
            r#"SELECT id, owner_id,
                      kind::text   AS kind,
                      issuing_org, issued_at, expires_at,
                      status::text AS status,
                      document_url, instructions, created_at, updated_at
               FROM licenses
               WHERE owner_id = $1
                 AND expires_at BETWEEN $2 AND $3
               ORDER BY expires_at ASC"#,
        )
        .bind(owner_id).bind(from).bind(to)
        .fetch_all(&self.pool).await?;

        rows.into_iter().map(row_to_license).collect()
    }

    async fn expiring_in_days_globally(
        &self,
        days_before: i32,
    ) -> DomainResult<Vec<License>> {
        let rows = sqlx::query(
            r#"SELECT id, owner_id,
                      kind::text   AS kind,
                      issuing_org, issued_at, expires_at,
                      status::text AS status,
                      document_url, instructions, created_at, updated_at
               FROM licenses
               WHERE status = 'active'
                 AND expires_at = (CURRENT_DATE + ($1 || ' days')::interval)::date
                 AND NOT EXISTS (
                     SELECT 1 FROM license_notifications
                     WHERE license_id = licenses.id AND days_before = $1
                 )"#,
        )
        .bind(days_before)
        .fetch_all(&self.pool).await?;

        rows.into_iter().map(row_to_license).collect()
    }

    async fn mark_notified(
        &self,
        license_id: Uuid,
        days_before: i32,
    ) -> DomainResult<bool> {
        // INSERT … ON CONFLICT DO NOTHING — claims the slot atomically
        // so that two notifier instances running in parallel can't double-send.
        let res = sqlx::query(
            r#"INSERT INTO license_notifications (license_id, days_before)
               VALUES ($1, $2)
               ON CONFLICT (license_id, days_before) DO NOTHING"#,
        )
        .bind(license_id).bind(days_before)
        .execute(&self.pool).await?;
        Ok(res.rows_affected() == 1)
    }

    async fn link_gun(
        &self,
        license_id: Uuid,
        gun_id: Uuid,
    ) -> DomainResult<()> {
        sqlx::query(
            r#"INSERT INTO license_gun_links (license_id, gun_id)
               VALUES ($1, $2)
               ON CONFLICT DO NOTHING"#,
        )
        .bind(license_id).bind(gun_id)
        .execute(&self.pool).await?;
        Ok(())
    }

    async fn unlink_gun(
        &self,
        license_id: Uuid,
        gun_id: Uuid,
    ) -> DomainResult<()> {
        sqlx::query(
            "DELETE FROM license_gun_links WHERE license_id=$1 AND gun_id=$2",
        )
        .bind(license_id).bind(gun_id)
        .execute(&self.pool).await?;
        Ok(())
    }

    async fn list_gun_links(&self, license_id: Uuid) -> DomainResult<Vec<Uuid>> {
        let rows = sqlx::query(
            "SELECT gun_id FROM license_gun_links WHERE license_id=$1",
        )
        .bind(license_id)
        .fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| r.get::<Uuid, _>("gun_id")).collect())
    }
}

fn row_to_license(r: PgRow) -> DomainResult<License> {
    let kind: LicenseKind = r.try_get::<String, _>("kind")?
        .parse().map_err(|e| DomainError::Infra(anyhow::anyhow!("bad license_kind: {e}")))?;
    let status: LicenseStatus = r.try_get::<String, _>("status")?
        .parse().map_err(|e| DomainError::Infra(anyhow::anyhow!("bad license_status: {e}")))?;
    Ok(License::rehydrate(
        r.try_get("id")?,
        r.try_get("owner_id")?,
        kind,
        r.try_get("issuing_org")?,
        r.try_get("issued_at")?,
        r.try_get("expires_at")?,
        status,
        r.try_get("document_url")?,
        r.try_get("instructions")?,
        r.try_get("created_at")?,
        r.try_get("updated_at")?,
    ))
}