// backend/src/models/audit_log.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AuditLog {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub session_id: Option<Uuid>,
    pub event_type: String,
    pub outcome: String,
    pub detail: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: time::OffsetDateTime,
}
