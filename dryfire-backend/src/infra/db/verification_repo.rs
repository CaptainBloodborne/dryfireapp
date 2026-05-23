use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{
    errors::{DomainError, DomainResult},
    repositories::user::VerificationRepository,
};

pub struct PgVerificationRepository {
    pool: PgPool,
}

impl PgVerificationRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl VerificationRepository for PgVerificationRepository {
    async fn create_email_verification(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> DomainResult<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO email_verifications
                (id, user_id, token_hash, expires_at, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    /// Consumes the token in a single statement so it can be replay-safe
    /// even under concurrent requests: the `consumed_at IS NULL`
    /// predicate together with `UPDATE ... RETURNING` is atomic.
    async fn consume_email_verification(
        &self,
        token_hash: &str,
    ) -> DomainResult<Uuid> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            UPDATE email_verifications
            SET consumed_at = NOW()
            WHERE token_hash = $1
              AND consumed_at IS NULL
              AND expires_at > NOW()
            RETURNING user_id
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|(uid,)| uid)
            .ok_or(DomainError::InvalidVerificationToken)
    }

    async fn create_password_reset(
        &self,
        user_id: Uuid,
        token_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> DomainResult<Uuid> {
        let id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO password_resets
                (id, user_id, token_hash, expires_at, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    async fn consume_password_reset(
        &self,
        token_hash: &str,
    ) -> DomainResult<Uuid> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            UPDATE password_resets
            SET consumed_at = NOW()
            WHERE token_hash = $1
              AND consumed_at IS NULL
              AND expires_at > NOW()
            RETURNING user_id
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|(uid,)| uid).ok_or(DomainError::InvalidResetToken)
    }
}
