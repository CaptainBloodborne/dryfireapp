//! Postgres implementation of [`UserRepository`].
//! 
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    domain::{
        entities::user::User,
        errors::{DomainError, DomainResult},
        repositories::user::UserRepository,
        services::identity::Credentials,
    },
    infra::db::models::UserRow,
};

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}


fn unique_violation_on(e: &sqlx::Error, col_hint: &str) -> bool {
    if let sqlx::Error::Database(db) = e {
        if db.code().as_deref() == Some("23505") {
            return db
                .constraint()
                .map(|c| c.contains(col_hint))
                .unwrap_or(false)
                || db.message().contains(col_hint);
        }
    }
    false
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn create_user(
        &self,
        user: &User,
        credentials: &Credentials,
    ) -> DomainResult<()> {

        let mut tx = self.pool.begin().await?;

        let res = sqlx::query(
            r#"
            INSERT INTO users
                (id, login, firstname, surname, email, date_of_birth,
                 region, language, status, last_visit_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::user_status, $10, $11, $12)
            "#,
        )
        .bind(user.id())
        .bind(user.login())
        .bind(user.firstname())
        .bind(user.surname())
        .bind(user.email())
        .bind(user.date_of_birth())
        .bind(user.region().as_str())
        .bind(user.language().as_str())
        .bind(user.status().as_str())
        .bind(user.last_visit_at())
        .bind(user.created_at())
        .bind(user.updated_at())
        .execute(&mut *tx)
        .await;

        if let Err(e) = res {
            return Err(if unique_violation_on(&e, "email") {
                DomainError::EmailAlreadyExists
            } else if unique_violation_on(&e, "login") {
                DomainError::LoginAlreadyTaken
            } else {
                DomainError::from(e)
            });
        }

        sqlx::query(
            r#"
            INSERT INTO user_credentials (user_id, password_hash, updated_at)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(user.id())
        .bind(credentials.password_hash())
        .bind(credentials.last_visit())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> DomainResult<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as(
            r#"
            SELECT id, login, firstname, surname, email, date_of_birth,
                   region, language, status::text AS status,
                   last_visit_at, created_at, updated_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(UserRow::into_domain).transpose()
    }

    async fn find_by_email(&self, email: &str) -> DomainResult<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as(
            r#"
            SELECT id, login, firstname, surname, email, date_of_birth,
                   region, language, status::text AS status,
                   last_visit_at, created_at, updated_at
            FROM users
            WHERE LOWER(email) = LOWER($1)
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        row.map(UserRow::into_domain).transpose()
    }

    async fn find_by_login(&self, login: &str) -> DomainResult<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as(
            r#"
            SELECT id, login, firstname, surname, email, date_of_birth,
                   region, language, status::text AS status,
                   last_visit_at, created_at, updated_at
            FROM users
            WHERE LOWER(login) = LOWER($1)
            "#,
        )
        .bind(login)
        .fetch_optional(&self.pool)
        .await?;
        row.map(UserRow::into_domain).transpose()
    }

    async fn find_credentials(
        &self,
        user_id: Uuid,
    ) -> DomainResult<Option<Credentials>> {
        let row: Option<(String, DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT password_hash, updated_at
            FROM user_credentials
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(hash, _)| Credentials::new(hash)))
    }

    async fn email_exists(&self, email: &str) -> DomainResult<bool> {
        let row = sqlx::query(
            "SELECT EXISTS (SELECT 1 FROM users WHERE LOWER(email) = LOWER($1)) AS exists",
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.try_get::<bool, _>("exists")?)
    }

    async fn login_exists(&self, login: &str) -> DomainResult<bool> {
        let row = sqlx::query(
            "SELECT EXISTS (SELECT 1 FROM users WHERE LOWER(login) = LOWER($1)) AS exists",
        )
        .bind(login)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.try_get::<bool, _>("exists")?)
    }

    async fn mark_verified(&self, user_id: Uuid) -> DomainResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET status = 'verified'::user_status,
                updated_at = NOW()
            WHERE id = $1
              AND status = 'pending'::user_status
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DomainError::AlreadyVerified);
        }
        Ok(())
    }

    async fn update_password_hash(
        &self,
        user_id: Uuid,
        new_hash: &str,
    ) -> DomainResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE user_credentials
            SET password_hash = $1, updated_at = NOW()
            WHERE user_id = $2
            "#,
        )
        .bind(new_hash)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DomainError::UserNotFound);
        }
        Ok(())
    }

    async fn touch_last_visit(
        &self,
        user_id: Uuid,
        when: DateTime<Utc>,
    ) -> DomainResult<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET last_visit_at = $1, updated_at = NOW()
            WHERE id = $2
            "#,
        )
        .bind(when)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
