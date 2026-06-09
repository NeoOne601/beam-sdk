// backend/src/models/webhook_config.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    pub secret_hex: Option<String>,
    pub active: bool,
    pub created_at: time::OffsetDateTime,
}
