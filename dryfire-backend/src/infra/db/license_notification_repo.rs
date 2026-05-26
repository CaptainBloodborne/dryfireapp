use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{
    errors::DomainResult,
    repositories::license::LicenseNotificationRepository,
};

pub struct PgLicenseNotificationRepository { pool: PgPool }

impl PgLicenseNotificationRepository {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl LicenseNotificationRepository for PgLicenseNotificationRepository {
    /// Insert with ON CONFLICT DO NOTHING. The unique (license_id,
    /// days_before) constraint makes this safe to call multiple times.
    async fn mark_sent(&self, license_id: Uuid, days_before: i32) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO license_notifications (license_id, days_before)
            VALUES ($1, $2)
            ON CONFLICT (license_id, days_before) DO NOTHING
            "#,
        )
        .bind(license_id).bind(days_before)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
