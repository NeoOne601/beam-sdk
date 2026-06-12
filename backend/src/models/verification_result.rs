// backend/src/models/verification_result.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct VerificationResult {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub session_id: Uuid,
    pub document_type: Option<String>,
    pub issuing_country: Option<String>,
    pub confidence: Option<f64>,
    pub pqc_verified: bool,
    pub pqc_public_key_hex: Option<String>,
    pub fraud_signals: Option<serde_json::Value>,
    pub created_at: time::OffsetDateTime,
}
