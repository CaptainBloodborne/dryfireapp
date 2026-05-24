// src/infra/db/law_repo.rs

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::{
    entities::law::{Law, LawTag},
    errors::{DomainError, DomainResult},
    repositories::{armory::PageQuery, law::LawRepository},
};

pub struct PgLawRepository {
    pool: PgPool,
}

impl PgLawRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl LawRepository for PgLawRepository {
    async fn upsert(&self, law: &Law) -> DomainResult<()> {
        let tags: Vec<&str> = law.tags.iter().map(|t| t.as_str()).collect();

        sqlx::query(
            r#"
            INSERT INTO laws
                (id, region, slug, title, body, version, tags,
                 published_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7::law_tag[],
                    $8, NOW())
            ON CONFLICT (region, slug, version) DO UPDATE
            SET title = EXCLUDED.title,
                body  = EXCLUDED.body,
                tags  = EXCLUDED.tags,
                updated_at = NOW()
            "#,
        )
        .bind(law.id)
        .bind(&law.region)
        .bind(&law.slug)
        .bind(&law.title)
        .bind(&law.body)
        .bind(law.version)
        .bind(&tags)
        .bind(law.published_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<Law>> {
        let row = sqlx::query(
            r#"SELECT id, region, slug, title, body, version,
                      ARRAY(SELECT t::text FROM unnest(tags) AS t) AS tags,
                      published_at, updated_at
               FROM laws
               WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool).await?;
        row.map(row_to_law).transpose()
    }

    async fn list(
        &self,
        region: Option<&str>,
        tags: &[LawTag],
        page: PageQuery,
    ) -> DomainResult<(Vec<Law>, i64)> {
        let tag_strs: Vec<&str> = tags.iter().map(|t| t.as_str()).collect();
        let tag_filter = !tag_strs.is_empty();

        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM laws
               WHERE ($1::text IS NULL OR region = $1)
                 AND ($2::bool = FALSE OR tags && $3::law_tag[])"#,
        )
        .bind(region)
        .bind(tag_filter)
        .bind(&tag_strs)
        .fetch_one(&self.pool).await?;

        let rows = sqlx::query(
            r#"SELECT id, region, slug, title, body, version,
                      ARRAY(SELECT t::text FROM unnest(tags) AS t) AS tags,
                      published_at, updated_at
               FROM laws
               WHERE ($1::text IS NULL OR region = $1)
                 AND ($2::bool = FALSE OR tags && $3::law_tag[])
               ORDER BY updated_at DESC
               LIMIT $4 OFFSET $5"#,
        )
        .bind(region)
        .bind(tag_filter)
        .bind(&tag_strs)
        .bind(page.limit())
        .bind(page.offset())
        .fetch_all(&self.pool).await?;

        let items = rows.into_iter()
            .map(row_to_law)
            .collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }

    async fn full_text(
        &self,
        region: Option<&str>,
        query: &str,
        page: PageQuery,
    ) -> DomainResult<(Vec<Law>, i64)> {
        let total: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM laws
               WHERE tsv @@ websearch_to_tsquery('russian', $1)
                 AND ($2::text IS NULL OR region = $2)"#,
        )
        .bind(query).bind(region)
        .fetch_one(&self.pool).await?;

        let rows = sqlx::query(
            r#"SELECT id, region, slug, title, body, version,
                      ARRAY(SELECT t::text FROM unnest(tags) AS t) AS tags,
                      published_at, updated_at,
                      ts_rank(tsv, websearch_to_tsquery('russian', $1)) AS rank
               FROM laws
               WHERE tsv @@ websearch_to_tsquery('russian', $1)
                 AND ($2::text IS NULL OR region = $2)
               ORDER BY rank DESC, updated_at DESC
               LIMIT $3 OFFSET $4"#,
        )
        .bind(query).bind(region)
        .bind(page.limit()).bind(page.offset())
        .fetch_all(&self.pool).await?;

        let items = rows.into_iter()
            .map(row_to_law)
            .collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }

    async fn updates_since(
        &self,
        region: Option<&str>,
        since: DateTime<Utc>,
    ) -> DomainResult<Vec<Law>> {
        let rows = sqlx::query(
            r#"SELECT id, region, slug, title, body, version,
                      ARRAY(SELECT t::text FROM unnest(tags) AS t) AS tags,
                      published_at, updated_at
               FROM laws
               WHERE updated_at > $1
                 AND ($2::text IS NULL OR region = $2)
               ORDER BY updated_at ASC"#,
        )
        .bind(since).bind(region)
        .fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_law).collect()
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        let res = sqlx::query("DELETE FROM laws WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 { return Err(DomainError::LawNotFound); }
        Ok(())
    }
}

fn row_to_law(r: PgRow) -> DomainResult<Law> {
    let tag_strs: Vec<String> = r.try_get("tags")?;
    let tags = tag_strs.iter()
        .map(|s| s.parse::<LawTag>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad law_tag: {e}")))?;
    Ok(Law {
        id: r.try_get("id")?,
        region: r.try_get("region")?,
        slug: r.try_get("slug")?,
        title: r.try_get("title")?,
        body: r.try_get("body")?,
        version: r.try_get("version")?,
        tags,
        published_at: r.try_get("published_at")?,
        updated_at: r.try_get("updated_at")?,
    })
}