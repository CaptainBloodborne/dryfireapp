use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use sqlx::{PgPool, Row};
use std::net::IpAddr;
use uuid::Uuid;

use crate::domain::{
    errors::DomainResult,
    repositories::user::{Session, SessionRepository},
};

pub struct PgSessionRepository {
    pool: PgPool,
}

impl PgSessionRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl SessionRepository for PgSessionRepository {
    async fn create(
        &self,
        user_id: Uuid,
        ttl_seconds: i64,
        user_agent: Option<&str>,
        ip: Option<IpAddr>,
    ) -> DomainResult<Session> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let expires = now + Duration::seconds(ttl_seconds);

        sqlx::query(
            r#"
            INSERT INTO user_sessions
                (id, user_id, created_at, expires_at, last_seen_at,
                 user_agent, ip_address)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(now)
        .bind(expires)
        .bind(now)
        .bind(user_agent)
        .bind(ip)
        .execute(&self.pool)
        .await?;

        Ok(Session {
            id,
            user_id,
            created_at: now,
            expires_at: expires,
            revoked_at: None,
        })
    }

    async fn find_active(
        &self,
        session_id: Uuid,
    ) -> DomainResult<Option<Session>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, created_at, expires_at, revoked_at
            FROM user_sessions
            WHERE id = $1
              AND revoked_at IS NULL
              AND expires_at > NOW()
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Session {
            id: r.get("id"),
            user_id: r.get("user_id"),
            created_at: r.get("created_at"),
            expires_at: r.get("expires_at"),
            revoked_at: r.get::<Option<DateTime<Utc>>, _>("revoked_at"),
        }))
    }

    async fn revoke(&self, session_id: Uuid) -> DomainResult<()> {
        sqlx::query(
            "UPDATE user_sessions SET revoked_at = NOW() WHERE id = $1 AND revoked_at IS NULL",
        )
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn revoke_all_for_user(&self, user_id: Uuid) -> DomainResult<()> {
        sqlx::query(
            "UPDATE user_sessions SET revoked_at = NOW() WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
