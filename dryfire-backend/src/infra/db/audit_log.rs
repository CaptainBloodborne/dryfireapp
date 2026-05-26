use async_trait::async_trait;
use sqlx::{PgPool, types::ipnetwork::IpNetwork};

use crate::domain::services::audit::{AuditEntry, AuditLogger};

pub struct PgAuditLogger { pool: PgPool }

impl PgAuditLogger {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl AuditLogger for PgAuditLogger {
    async fn record(&self, entry: AuditEntry) {
        let ip_db: Option<IpNetwork> = entry.ip_address.map(IpNetwork::from);

        let result = sqlx::query(
            r#"
            INSERT INTO audit_log
              (user_id, action, resource_type, resource_id,
               request_id, ip_address, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(entry.user_id)
        .bind(&entry.action)
        .bind(entry.resource_type.as_deref())
        .bind(entry.resource_id.as_deref())
        .bind(entry.request_id.as_deref())
        .bind(ip_db)
        .bind(entry.metadata)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            // Never escalate audit failures — log and move on.
            tracing::error!(
                error = ?e,
                action = %entry.action,
                "audit log write failed"
            );
        }
    }
}
