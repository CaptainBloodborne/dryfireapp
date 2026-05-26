//! Postgres implementation of [`LawRepository`].
//!
//! Implementation notes:
//!
//! - **Tags round-trip via `Vec<String>`.** Postgres `TEXT[]` decodes
//!   to `Vec<String>` in SQLx; we then map each element through
//!   `LawTag::from_str`. Failures are mapped to `DomainError::Infra`
//!   (a bad tag in the DB is a data-integrity bug, not a user error).
//! - **`update_by_key` does NOT manage versioning in app code.** The
//!   DB trigger `laws_snapshot_on_update` handles snapshotting +
//!   incrementing `current_version` atomically. We then re-read the
//!   row to learn the new version number.
//! - **Search uses `websearch_to_tsquery`** rather than `plainto_tsquery`
//!   so users can write quoted phrases (`"короткоствольное оружие"`),
//!   negations (`-самооборона`), and boolean operators.
//! - **Snippet generation via `ts_headline`** with `<mark>` wrappers
//!   (HTML-safe — angle brackets in the body are escaped by Postgres).

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::domain::{
    entities::law::{Law, LawCategory, LawSearchHit, LawTag, LawVersion},
    errors::{DomainError, DomainResult},
    repositories::law::{LawFilter, LawRepository},
};

pub struct PgLawRepository { pool: PgPool }

impl PgLawRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

fn unique_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db) if db.code().as_deref() == Some("23505"))
}

/// Map a Postgres TEXT[] of tag codes into typed `LawTag`s.
fn parse_tags(strs: Vec<String>) -> DomainResult<Vec<LawTag>> {
    strs.into_iter()
        .map(|s| LawTag::from_str(&s)
            .map_err(|e| DomainError::Infra(anyhow::anyhow!("bad tag in DB: {e}"))))
        .collect()
}

fn tags_to_strs(tags: &[LawTag]) -> Vec<&'static str> {
    tags.iter().map(LawTag::as_str).collect()
}

fn category_row(r: PgRow) -> LawCategory {
    LawCategory {
        id: r.get("id"),
        code: r.get("code"),
        name: r.get("name"),
        parent_id: r.get::<Option<Uuid>, _>("parent_id"),
        sort_order: r.get("sort_order"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    }
}

fn law_row(r: PgRow) -> DomainResult<Law> {
    let tags_strs: Vec<String> = r.get("tags");
    Ok(Law {
        id: r.get("id"),
        law_key: r.get("law_key"),
        title: r.get("title"),
        summary: r.get::<Option<String>, _>("summary"),
        body: r.get("body"),
        region: r.get("region"),
        category_id: r.get::<Option<Uuid>, _>("category_id"),
        tags: parse_tags(tags_strs)?,
        current_version: r.get("current_version"),
        effective_at: r.get::<NaiveDate, _>("effective_at"),
        created_at: r.get::<DateTime<Utc>, _>("created_at"),
        updated_at: r.get::<DateTime<Utc>, _>("updated_at"),
    })
}

fn version_row(r: PgRow) -> DomainResult<LawVersion> {
    let tags_strs: Vec<String> = r.get("tags");
    Ok(LawVersion {
        id: r.get("id"),
        law_id: r.get("law_id"),
        law_key: r.get("law_key"),
        version: r.get("version"),
        title: r.get("title"),
        summary: r.get::<Option<String>, _>("summary"),
        body: r.get("body"),
        tags: parse_tags(tags_strs)?,
        category_id: r.get::<Option<Uuid>, _>("category_id"),
        effective_at: r.get::<NaiveDate, _>("effective_at"),
        snapshot_at: r.get::<DateTime<Utc>, _>("snapshot_at"),
    })
}

/// Columns commonly selected for a Law row. Excludes `search_vector`
/// (which is large and not needed by the app).
const LAW_COLS: &str =
    "id, law_key, title, summary, body, region, category_id, tags, \
     current_version, effective_at, created_at, updated_at";

#[async_trait]
impl LawRepository for PgLawRepository {
    // categories

    async fn category_create(&self, c: &LawCategory) -> DomainResult<()> {
        let res = sqlx::query(
            r#"
            INSERT INTO law_categories
              (id, code, name, parent_id, sort_order, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(c.id).bind(&c.code).bind(&c.name)
        .bind(c.parent_id).bind(c.sort_order)
        .bind(c.created_at).bind(c.updated_at)
        .execute(&self.pool).await;

        if let Err(e) = res {
            if unique_violation(&e) {
                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        "law category with this code already exists".into())));
            }
            return Err(DomainError::from(e));
        }
        Ok(())
    }

    async fn category_update(&self, c: &LawCategory) -> DomainResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE law_categories SET
              code = $1, name = $2, parent_id = $3, sort_order = $4,
              updated_at = NOW()
            WHERE id = $5
            "#,
        )
        .bind(&c.code).bind(&c.name).bind(c.parent_id)
        .bind(c.sort_order).bind(c.id)
        .execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::LawNotFound);
        }
        Ok(())
    }

    async fn category_delete(&self, id: Uuid) -> DomainResult<()> {
        let result = sqlx::query("DELETE FROM law_categories WHERE id = $1")
            .bind(id).execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::LawNotFound);
        }
        Ok(())
    }

    async fn category_find(&self, id: Uuid) -> DomainResult<Option<LawCategory>> {
        let row = sqlx::query(
            "SELECT id, code, name, parent_id, sort_order, created_at, updated_at \
             FROM law_categories WHERE id = $1",
        )
        .bind(id).fetch_optional(&self.pool).await?;
        Ok(row.map(category_row))
    }

    async fn category_list(&self) -> DomainResult<Vec<LawCategory>> {
        let rows = sqlx::query(
            "SELECT id, code, name, parent_id, sort_order, created_at, updated_at \
             FROM law_categories \
             ORDER BY sort_order ASC, name ASC",
        ).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(category_row).collect())
    }

    // laws

    async fn create(&self, law: &Law) -> DomainResult<()> {
        let tag_strs = tags_to_strs(&law.tags);
        let res = sqlx::query(
            r#"
            INSERT INTO laws
              (id, law_key, title, summary, body, region,
               category_id, tags, current_version, effective_at,
               created_at, updated_at)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            "#,
        )
        .bind(law.id).bind(&law.law_key).bind(&law.title)
        .bind(&law.summary).bind(&law.body).bind(&law.region)
        .bind(law.category_id).bind(&tag_strs)
        .bind(law.current_version).bind(law.effective_at)
        .bind(law.created_at).bind(law.updated_at)
        .execute(&self.pool).await;

        if let Err(e) = res {
            if unique_violation(&e) {
                return Err(DomainError::Validation(
                    crate::domain::errors::ValidationError::Custom(
                        "law with this law_key already exists".into())));
            }
            return Err(DomainError::from(e));
        }
        Ok(())
    }

    async fn update_by_key(&self, law: &Law) -> DomainResult<Law> {
        let tag_strs = tags_to_strs(&law.tags);

        let row = sqlx::query(
            format!(
                "UPDATE laws SET \
                   title = $1, summary = $2, body = $3, region = $4, \
                   category_id = $5, tags = $6, effective_at = $7 \
                 WHERE law_key = $8 \
                 RETURNING {LAW_COLS}",
            ).as_str(),
        )
        .bind(&law.title).bind(&law.summary).bind(&law.body)
        .bind(&law.region).bind(law.category_id).bind(&tag_strs)
        .bind(law.effective_at).bind(&law.law_key)
        .fetch_optional(&self.pool).await?;

        let r = row.ok_or(DomainError::LawNotFound)?;
        law_row(r)
    }

    async fn delete(&self, id: Uuid) -> DomainResult<()> {
        let result = sqlx::query("DELETE FROM laws WHERE id = $1")
            .bind(id).execute(&self.pool).await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::LawNotFound);
        }
        Ok(())
    }

    async fn find(&self, id: Uuid) -> DomainResult<Option<Law>> {
        let row = sqlx::query(
            format!("SELECT {LAW_COLS} FROM laws WHERE id = $1").as_str(),
        )
        .bind(id).fetch_optional(&self.pool).await?;
        row.map(law_row).transpose()
    }

    async fn find_by_key(&self, law_key: &str) -> DomainResult<Option<Law>> {
        let row = sqlx::query(
            format!("SELECT {LAW_COLS} FROM laws WHERE law_key = $1").as_str(),
        )
        .bind(law_key).fetch_optional(&self.pool).await?;
        row.map(law_row).transpose()
    }

    async fn list(
        &self,
        filter: &LawFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<Law>, i64)> {
        let (where_sql, bind_count) = build_law_where(filter, 1);

        // count
        let count_sql = format!("SELECT COUNT(*) AS c FROM laws {where_sql}");
        let cq = bind_law_filter(sqlx::query(&count_sql), filter);
        let total: i64 = cq.fetch_one(&self.pool).await?.get("c");

        // page
        let sel_sql = format!(
            "SELECT {LAW_COLS} FROM laws {where_sql} \
             ORDER BY updated_at DESC \
             LIMIT ${l_idx} OFFSET ${o_idx}",
            l_idx = bind_count, o_idx = bind_count + 1,
        );
        let mut q = bind_law_filter(sqlx::query(&sel_sql), filter);
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        let items = rows.into_iter().map(law_row).collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }

    async fn search(
        &self,
        query_text: &str,
        filter: &LawFilter,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<LawSearchHit>, i64)> {
        
        let q_param = 1;
        let (filter_where, after_filter_idx) = build_law_where(filter, 2);

        let where_sql = format!(
            "WHERE search_vector @@ websearch_to_tsquery('russian', ${q_param}) \
             {extra}",
            extra = if filter_where.is_empty() { "".to_string() }
                    else { filter_where.replacen("WHERE", "AND", 1) },
        );

        // count
        let count_sql = format!("SELECT COUNT(*) AS c FROM laws {where_sql}");
        let mut cq = sqlx::query(&count_sql).bind(query_text);
        cq = bind_law_filter(cq, filter);
        let total: i64 = cq.fetch_one(&self.pool).await?.get("c");

        // page
        // ts_rank_cd cares about position-density and cover-density;
        // gives more meaningful ranking than ts_rank for legal text.
        let sel_sql = format!(
            "SELECT {LAW_COLS}, \
                    ts_rank_cd(search_vector, websearch_to_tsquery('russian', ${q_param})) AS rank, \
                    ts_headline('russian', body, \
                                websearch_to_tsquery('russian', ${q_param}), \
                                'StartSel=<mark>, StopSel=</mark>, MaxFragments=3, FragmentDelimiter=…') AS snippet \
             FROM laws \
             {where_sql} \
             ORDER BY rank DESC, updated_at DESC \
             LIMIT ${l_idx} OFFSET ${o_idx}",
            l_idx = after_filter_idx,
            o_idx = after_filter_idx + 1,
        );
        let mut q = sqlx::query(&sel_sql).bind(query_text);
        q = bind_law_filter(q, filter);
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            let rank: f32 = r.get("rank");
            let snippet: String = r.get("snippet");
            // Build the Law from the same row.
            let law = law_row(r)?;
            items.push(LawSearchHit { law, rank, snippet });
        }
        Ok((items, total))
    }

    // history

    async fn versions(&self, law_id: Uuid) -> DomainResult<Vec<LawVersion>> {
        let rows = sqlx::query(
            r#"
            SELECT id, law_id, law_key, version, title, summary, body,
                   tags, category_id, effective_at, snapshot_at
            FROM law_versions
            WHERE law_id = $1
            ORDER BY version DESC
            "#,
        )
        .bind(law_id).fetch_all(&self.pool).await?;
        rows.into_iter().map(version_row).collect()
    }

    async fn changes_since(
        &self,
        region: &str,
        since: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> DomainResult<(Vec<Law>, i64)> {
        let total: i64 = sqlx::query(
            "SELECT COUNT(*) AS c FROM laws \
             WHERE region = $1 AND updated_at > $2",
        )
        .bind(region).bind(since)
        .fetch_one(&self.pool).await?
        .get("c");

        let rows = sqlx::query(
            format!(
                "SELECT {LAW_COLS} FROM laws \
                 WHERE region = $1 AND updated_at > $2 \
                 ORDER BY updated_at DESC \
                 LIMIT $3 OFFSET $4",
            ).as_str(),
        )
        .bind(region).bind(since).bind(limit).bind(offset)
        .fetch_all(&self.pool).await?;

        let items = rows.into_iter().map(law_row).collect::<DomainResult<Vec<_>>>()?;
        Ok((items, total))
    }
}

//  filter SQL helpers

/// Build the WHERE clause and the next bind-parameter index.
/// `start_idx` is the parameter number for the first filter bind.
fn build_law_where(filter: &LawFilter, start_idx: i32) -> (String, i32) {
    let mut sql = String::from("WHERE TRUE");
    let mut idx = start_idx;

    if filter.region.is_some() {
        sql.push_str(&format!(" AND region = ${idx}")); idx += 1;
    }
    if filter.category_id.is_some() {
        sql.push_str(&format!(" AND category_id = ${idx}")); idx += 1;
    }
    if !filter.any_tags.is_empty() {
        // `tags && ARRAY[...]` — overlap (OR semantics).
        sql.push_str(&format!(" AND tags && ${idx}")); idx += 1;
    }
    if !filter.all_tags.is_empty() {
        // `tags @> ARRAY[...]` — contains (AND semantics).
        sql.push_str(&format!(" AND tags @> ${idx}")); idx += 1;
    }
    if filter.updated_after.is_some() {
        sql.push_str(&format!(" AND updated_at > ${idx}")); idx += 1;
    }
    if filter.effective_after.is_some() {
        sql.push_str(&format!(" AND effective_at >= ${idx}")); idx += 1;
    }

    if sql == "WHERE TRUE" {
        // Caller may want to add their own conditions; an empty WHERE
        // is more convenient than `WHERE TRUE`.
        (String::new(), idx)
    } else {
        (sql, idx)
    }
}

fn bind_law_filter<'a>(
    mut q: sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments>,
    filter: &'a LawFilter,
) -> sqlx::query::Query<'a, sqlx::Postgres, sqlx::postgres::PgArguments> {
    if let Some(r) = &filter.region { q = q.bind(r); }
    if let Some(c) = filter.category_id { q = q.bind(c); }
    if !filter.any_tags.is_empty() {
        let strs: Vec<&str> = filter.any_tags.iter().map(LawTag::as_str).collect();
        q = q.bind(strs);
    }
    if !filter.all_tags.is_empty() {
        let strs: Vec<&str> = filter.all_tags.iter().map(LawTag::as_str).collect();
        q = q.bind(strs);
    }
    if let Some(u) = filter.updated_after { q = q.bind(u); }
    if let Some(e) = filter.effective_after { q = q.bind(e); }
    q
}
