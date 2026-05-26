//! Audit log port. Use cases call this for every state-changing
//! operation. The implementation writes to the `audit_log` table.
//!
//! Failure to write an audit entry is **logged but not propagated** —
//! we don't refuse a user's gun deletion because the audit row failed
//! to insert (that would be worse). The tracing layer captures the
//! failure for ops to investigate.

use async_trait::async_trait;
use serde_json::Value;
use std::net::IpAddr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub request_id: Option<String>,
    pub ip_address: Option<IpAddr>,
    pub metadata: Option<Value>,
}

impl AuditEntry {
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            user_id: None,
            action: action.into(),
            resource_type: None,
            resource_id: None,
            request_id: None,
            ip_address: None,
            metadata: None,
        }
    }
    pub fn user(mut self, id: Uuid) -> Self { self.user_id = Some(id); self }
    pub fn resource(mut self, ty: impl Into<String>, id: impl ToString) -> Self {
        self.resource_type = Some(ty.into());
        self.resource_id = Some(id.to_string());
        self
    }
    pub fn metadata(mut self, m: Value) -> Self { self.metadata = Some(m); self }
}

#[async_trait]
pub trait AuditLogger: Send + Sync {
    async fn record(&self, entry: AuditEntry);
}
