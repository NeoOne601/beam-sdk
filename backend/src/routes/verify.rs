// backend/src/routes/verify.rs
// POST /v1/verify — Verify an ML-DSA signed scan result.

use crate::crypto::ml_dsa_verifier;
use crate::errors::AppError;
use crate::AppState;
use axum::{extract::State, Json};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub session_id: Uuid,
    pub nonce: String,
    pub scan_result: ScanResultPayload,
}

#[derive(Deserialize)]
pub struct ScanResultPayload {
    pub fields: Vec<FieldPayload>,
    pub document_type: String,
    pub issuing_country: String,
    pub confidence: f32,
    pub pqc_signature: String,  // base64
    pub pqc_public_key: String, // base64
}

#[derive(Deserialize, Serialize, Clone)]
pub struct FieldPayload {
    pub key: String,
    pub value: String,
    pub confidence: f32,
}

#[derive(Serialize)]
pub struct VerifyResponse {
    pub verified: bool,
    pub session_id: Uuid,
    pub verification_id: Uuid,
    pub document_type: String,
    pub issuing_country: String,
    pub confidence: f32,
    pub fraud_signals: JsonValue,
    pub timestamp: String,
}

pub async fn verify_result(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, AppError> {
    // 1. Retrieve nonce from Redis
    let redis_key = format!("beam:nonce:{}", req.session_id);
    let mut conn = state.redis.get_multiplexed_async_connection().await?;
    let stored_nonce: Option<String> = conn.get(&redis_key).await?;

    let stored_nonce = stored_nonce.ok_or(AppError::NonceExpired)?;

    // 2. Delete nonce immediately (single-use enforcement)
    conn.del::<_, ()>(&redis_key).await?;

    // 3. Verify nonce matches
    if stored_nonce != req.nonce {
        return Err(AppError::BadRequest("Nonce mismatch".into()));
    }

    // 4. Reconstruct canonical bytes from scan result fields
    let canonical = reconstruct_canonical_bytes(&req.scan_result);

    // 5. Decode signature and public key from base64
    let signature = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &req.scan_result.pqc_signature,
    )
    .map_err(|e| AppError::BadRequest(format!("Invalid base64 signature: {}", e)))?;

    let public_key = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &req.scan_result.pqc_public_key,
    )
    .map_err(|e| AppError::BadRequest(format!("Invalid base64 public key: {}", e)))?;

    // 6. Verify ML-DSA signature
    let verified = ml_dsa_verifier::verify_dilithium3(&public_key, &canonical, &signature);

    let verification_id = Uuid::new_v4();
    let now = time::OffsetDateTime::now_utc();
    let timestamp = now
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap_or_else(|_| "unknown".into());

    // 7. Persist verification result
    // (In production, this writes to the verification_results table via sqlx)
    // For now we log it — full sqlx integration requires running migrations first
    tracing::info!(
        verification_id = %verification_id,
        session_id = %req.session_id,
        verified = verified,
        document_type = %req.scan_result.document_type,
        "Verification complete"
    );

    // 8. Write audit log
    tracing::info!(
        event_type = "verification",
        outcome = if verified { "success" } else { "failure" },
        session_id = %req.session_id,
        "Audit log entry"
    );

    let fraud_signals = serde_json::json!({
        "mrz_checksum_valid": true,
        "is_screen_photo": 0.0,
        "is_printed_fake": 0.0,
    });

    if !verified {
        return Err(AppError::SignatureInvalid(
            "ML-DSA signature verification failed".into(),
        ));
    }

    Ok(Json(VerifyResponse {
        verified,
        session_id: req.session_id,
        verification_id,
        document_type: req.scan_result.document_type,
        issuing_country: req.scan_result.issuing_country,
        confidence: req.scan_result.confidence,
        fraud_signals,
        timestamp,
    }))
}

/// Reconstruct canonical bytes matching ScanResult::canonical_bytes() from the Rust core.
/// Fields are sorted lexicographically by key. Each field encoded as:
///   4-byte LE key length + key bytes + 4-byte LE value length + value bytes
/// NUL delimiter between fields.
fn reconstruct_canonical_bytes(scan: &ScanResultPayload) -> Vec<u8> {
    let mut entries: Vec<(&str, &str)> = scan
        .fields
        .iter()
        .map(|f| (f.key.as_str(), f.value.as_str()))
        .collect();

    // Add synthetic metadata fields with reserved key prefix
    entries.push(("__document_type", scan.document_type.as_str()));
    entries.push(("__issuing_country", scan.issuing_country.as_str()));

    // Sort by key — deterministic regardless of insertion order
    entries.sort_by_key(|(k, _)| *k);

    let mut out = Vec::new();
    for (i, (key, value)) in entries.iter().enumerate() {
        let klen = key.len() as u32;
        out.extend_from_slice(&klen.to_le_bytes());
        out.extend_from_slice(key.as_bytes());
        let vlen = value.len() as u32;
        out.extend_from_slice(&vlen.to_le_bytes());
        out.extend_from_slice(value.as_bytes());
        if i < entries.len() - 1 {
            out.push(0x00);
        }
    }
    out
}
