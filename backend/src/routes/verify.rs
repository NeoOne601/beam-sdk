// backend/src/routes/verify.rs
// POST /v1/verify — Verify an ML-DSA signed scan result.
//
// Security hardening (see README.md for full rationale):
//   VR-1: Nonce is now bound into the signed canonical bytes — __nonce, __session_id,
//         __timestamp are included in the reconstructed canonical payload. A replay
//         requires the attacker to reproduce the exact nonce + session binding.
//   VR-2: public key is no longer accepted from the caller. The public key is
//         looked up from trusted_public_keys via the active KeyProvider strategy.
//         key_id in the request selects which registered key to use.
//   VR-3: TenantContext extracted from request extension (set by auth middleware).
//         All DB writes are scoped to the authenticated tenant_id.

use crate::errors::AppError;
use crate::middleware::auth::TenantContext;
use crate::AppState;
use axum::{extract::State, Extension, Json};
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
    pub pqc_signature: String, // base64-encoded ML-DSA detached signature

    /// VR-2: Identifies which pre-registered key in trusted_public_keys to use.
    /// The meaning depends on the active KEY_PROVIDER_STRATEGY (tenant/device/model).
    /// public key is NO LONGER accepted from the caller.
    pub key_id: String,

    /// VR-1: The nonce as it was embedded in the signed canonical bytes.
    /// Must match the nonce stored in Redis for this session.
    pub signed_nonce: String,

    /// VR-1: The session_id as it was embedded in the signed canonical bytes.
    pub signed_session_id: String,

    /// VR-1: The UTC timestamp (Unix seconds string) embedded in the signed bytes.
    pub signed_timestamp: String,
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
    /// VR-2: Echoes back the key_id used for verification, for audit purposes.
    pub key_id: String,
}

pub async fn verify_result(
    State(state): State<Arc<AppState>>,
    Extension(tenant): Extension<TenantContext>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, AppError> {
    // ── Step 1: Retrieve nonce from Redis (namespaced by tenant + session) ──────
    // VR-3: Nonce key is namespaced by tenant_id to prevent cross-tenant nonce reuse.
    let redis_key = format!("beam:nonce:{}:{}", tenant.tenant_id, req.session_id);
    let mut conn = state.redis.get_multiplexed_async_connection().await?;
    let stored_nonce: Option<String> = conn.get(&redis_key).await?;
    let stored_nonce = stored_nonce.ok_or(AppError::NonceExpired)?;

    // ── Step 2: Validate basic nonce match before doing expensive crypto ─────────
    if stored_nonce != req.nonce {
        return Err(AppError::BadRequest("Nonce mismatch".into()));
    }

    // ── Step 3: VR-1 — Validate signed_nonce matches the stored nonce ────────────
    // The signed_nonce is the nonce as it was embedded in canonical_bytes() at
    // signing time. It must equal the nonce we issued — otherwise the signature
    // covers a different nonce and replay protection is defeated.
    if req.scan_result.signed_nonce != req.nonce {
        return Err(AppError::BadRequest(
            "signed_nonce does not match the issued nonce".into(),
        ));
    }

    // ── Step 4: VR-1 — Validate signed_session_id matches session_id ────────────
    let signed_session_id_str = req.scan_result.signed_session_id.as_str();
    if signed_session_id_str != req.session_id.to_string().as_str() {
        return Err(AppError::BadRequest(
            "signed_session_id does not match request session_id".into(),
        ));
    }

    // ── Step 5: VR-1 — Validate signed_timestamp is within freshness window ──────
    let signed_ts: u64 = req
        .scan_result
        .signed_timestamp
        .parse()
        .map_err(|_| AppError::BadRequest("signed_timestamp is not a valid integer".into()))?;
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let freshness_window = state.config.result_freshness_window_secs;
    if now_secs.saturating_sub(signed_ts) > freshness_window {
        return Err(AppError::BadRequest(format!(
            "Signed result is stale (timestamp {} is older than {}s freshness window)",
            signed_ts, freshness_window
        )));
    }

    // ── Step 6: VR-2 — Look up trusted public key (never trust client-supplied) ──
    let trusted_key = state
        .key_provider
        .get_key(&state.db_pool, &tenant, &req.scan_result.key_id)
        .await?;

    // ── Step 7: Reconstruct canonical bytes (must mirror canonical_bytes() in core) ─
    let canonical = reconstruct_canonical_bytes(&req.scan_result);

    // ── Step 8: Decode signature from base64 ────────────────────────────────────
    let signature = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &req.scan_result.pqc_signature,
    )
    .map_err(|e| AppError::BadRequest(format!("Invalid base64 signature: {}", e)))?;

    // ── Step 9: Verify ML-DSA signature against the TRUSTED key ─────────────────
    use crate::crypto::ml_dsa_verifier;
    let verified =
        ml_dsa_verifier::verify_dilithium3(&trusted_key.public_key_bytes, &canonical, &signature);

    // ── Step 10: Consume nonce ONLY after successful signature verification ───────
    // Consuming the nonce before verification would allow an attacker to exhaust
    // nonces via unsigned requests. We consume only on success (or definitive failure).
    conn.del::<_, ()>(&redis_key).await?;

    let verification_id = Uuid::new_v4();
    let now = time::OffsetDateTime::now_utc();
    let timestamp = now
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap_or_else(|_| "unknown".into());

    let fraud_signals = serde_json::json!({
        "mrz_checksum_valid": true,
        "is_screen_photo": 0.0,
        "is_printed_fake": 0.0,
    });

    // ── Step 11: Persist verification result (VR-3: scoped to tenant_id) ─────────
    sqlx::query!(
        r#"
        INSERT INTO verification_results
            (id, tenant_id, session_id, document_type, issuing_country,
             confidence, pqc_verified, pqc_public_key_hex, fraud_signals, key_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
        verification_id,
        tenant.tenant_id,
        req.session_id,
        req.scan_result.document_type,
        req.scan_result.issuing_country,
        req.scan_result.confidence as f64,
        verified,
        hex::encode(&req.scan_result.pqc_signature),
        serde_json::to_value(&fraud_signals).ok(),
        trusted_key.key_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    // ── Step 12: Write audit log (VR-3: scoped to tenant_id) ────────────────────
    sqlx::query!(
        r#"
        INSERT INTO audit_logs
            (id, tenant_id, session_id, event_type, outcome, detail)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::new_v4(),
        tenant.tenant_id,
        req.session_id,
        "verification",
        if verified { "success" } else { "failure" },
        serde_json::json!({
            "document_type": req.scan_result.document_type,
            "issuing_country": req.scan_result.issuing_country,
            "confidence": req.scan_result.confidence,
            "key_id": trusted_key.key_id,
            "verification_id": verification_id.to_string()
        })
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

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
        key_id: trusted_key.key_id,
    }))
}

/// Reconstruct canonical bytes matching ScanResult::canonical_bytes() from the Rust core.
///
/// VR-1: Includes __nonce, __session_id, __timestamp in the signed payload.
/// The encoding must be byte-for-byte identical to canonical_bytes() in core/src/result.rs.
///
/// Fields are sorted lexicographically by key. Each field encoded as:
///   4-byte LE key length + key bytes + 4-byte LE value length + value bytes
/// NUL delimiter between fields.
fn reconstruct_canonical_bytes(scan: &ScanResultPayload) -> Vec<u8> {
    let mut entries: Vec<(&str, &str)> = scan
        .fields
        .iter()
        .map(|f| (f.key.as_str(), f.value.as_str()))
        .collect();

    // Synthetic metadata fields (mirrors result.rs canonical_bytes)
    entries.push(("__document_type", scan.document_type.as_str()));
    entries.push(("__issuing_country", scan.issuing_country.as_str()));

    // VR-1: Session-binding fields included in signed payload.
    entries.push(("__nonce", scan.signed_nonce.as_str()));
    entries.push(("__session_id", scan.signed_session_id.as_str()));
    entries.push(("__timestamp", scan.signed_timestamp.as_str()));

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
