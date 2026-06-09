// backend/src/models/nonce.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct NonceRecord {
    pub session_id: Uuid,
    pub nonce_hex: String,
    pub expires_at: time::OffsetDateTime,
}
