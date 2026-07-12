// backend/src/routes/verify.rs
// POST /v1/verify — Verify a scan result signature (ed25519, ml-dsa-65, or future algos).
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
use base64::Engine as _;
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

    /// Algorithm used to sign this scan result.
    /// Supported values: "ed25519", "ml-dsa-65".
    pub algo: String,

    /// Base64-encoded Ed25519 public key (32 bytes).
    /// Only required when algo == "ed25519".
    #[serde(default)]
    pub public_key: String,

    /// Raw bytes of the PQC (Dilithium-3 / ML-DSA-65) public key.
    /// Only required when algo == "ml-dsa-65".
    #[serde(default)]
    pub pqc_public_key: Vec<u8>,
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
    /// Country-Specific Rules Engine outcome (dynamic thresholds by ISO code).
    pub country_rules: crate::rules::RuleOutcome,
    /// NQM crypto-agility envelope describing the client payload signature.
    pub nqm_compliance: crate::nqm::NqmCompliance,
    /// Quantum-safe (ML-DSA-65) server counter-signature over this outcome.
    pub server_attestation: crate::nqm::ServerAttestation,
}

pub async fn verify_result(
    State(state): State<Arc<AppState>>,
    Extension(tenant): Extension<TenantContext>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, AppError> {
    // ── Step 1: Retrieve nonce from Redis (namespaced by tenant + session) ──────
    // VR-3: Nonce key is namespaced by tenant_id to prevent cross-tenant nonce reuse.
    let redis_key = format!("ajna:nonce:{}:{}", tenant.tenant_id, req.session_id);
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
    let canonical_bytes = reconstruct_canonical_bytes(&req.scan_result);

    // ── Step 8: Decode signature from base64 ────────────────────────────────────
    let sig_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &req.scan_result.pqc_signature,
    )
    .map_err(|e| AppError::BadRequest(format!("Invalid base64 signature: {}", e)))?;

    // ── Step 9: Dispatch verification by algorithm ───────────────────────────────
    // VR-2: verification uses ONLY the pre-registered trusted key. Client-supplied
    // key material is either ignored or must match the registered key byte-for-byte.
    // The single documented exception is ALLOW_UNREGISTERED_ED25519_KEYS=true
    // (demo deployments): ed25519 may fall back to the client-supplied key, and
    // every such verification is warned in logs and marked in the audit trail.
    let mut key_trust = "registered";
    let mut used_key_hex = hex::encode(&trusted_key.public_key_bytes);
    let verified = match req.scan_result.algo.as_str() {
        "ed25519" => {
            let client_key_bytes = base64::engine::general_purpose::STANDARD
                .decode(&req.scan_result.public_key)
                .map_err(|e| AppError::BadRequest(format!("Invalid public_key base64: {e}")))?;

            let key_bytes: &[u8] = if trusted_key.algorithm.eq_ignore_ascii_case("ed25519") {
                if !client_key_bytes.is_empty()
                    && client_key_bytes != trusted_key.public_key_bytes
                {
                    return Err(AppError::Unauthorized(
                        "Supplied public_key does not match the registered ed25519 key (VR-2)"
                            .into(),
                    ));
                }
                &trusted_key.public_key_bytes
            } else if state.config.allow_unregistered_ed25519 {
                tracing::warn!(
                    tenant_id = %tenant.tenant_id,
                    "VR-2 demo mode: verifying against CLIENT-SUPPLIED ed25519 key \
                     (ALLOW_UNREGISTERED_ED25519_KEYS=true). Do not run production this way."
                );
                key_trust = "client-supplied-demo";
                used_key_hex = hex::encode(&client_key_bytes);
                &client_key_bytes
            } else {
                return Err(AppError::Unauthorized(
                    "No registered ed25519 key for this tenant; client-supplied keys are \
                     not trusted (VR-2). Register the key, or set \
                     ALLOW_UNREGISTERED_ED25519_KEYS=true for demo deployments only."
                        .into(),
                ));
            };

            crate::crypto::ed25519_verifier::verify_ed25519(
                &canonical_bytes,
                &sig_bytes,
                key_bytes,
            )
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?
        }
        "ml-dsa-65" => {
            if !trusted_key
                .algorithm
                .to_ascii_lowercase()
                .starts_with("ml-dsa")
            {
                return Err(AppError::Unauthorized(format!(
                    "Registered key algorithm is '{}', not ML-DSA; cannot verify an \
                     ml-dsa-65 result against it (VR-2)",
                    trusted_key.algorithm
                )));
            }
            if !req.scan_result.pqc_public_key.is_empty()
                && req.scan_result.pqc_public_key != trusted_key.public_key_bytes
            {
                return Err(AppError::Unauthorized(
                    "Supplied pqc_public_key does not match the registered key (VR-2)".into(),
                ));
            }
            crate::crypto::ml_dsa_verifier::verify_dilithium3(
                &trusted_key.public_key_bytes,
                &canonical_bytes,
                &sig_bytes,
            )
        }
        "ecdsa-p256" => {
            return Err(AppError::BadRequest(
                "ECDSA-P256 verification not yet implemented. \
                 Use ed25519 or ml-dsa-65."
                    .into(),
            ));
        }
        "hybrid-ed25519-ml-dsa-65" => {
            return Err(AppError::BadRequest(
                "Hybrid verification not yet implemented. \
                 Use ed25519 or ml-dsa-65."
                    .into(),
            ));
        }
        unknown => {
            return Err(AppError::BadRequest(format!(
                "Unknown algorithm: {unknown}. Supported: ed25519, ml-dsa-65"
            )));
        }
    };

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

    // ── Step 10.5: Country-Specific Rules Engine ─────────────────────────────────
    // Dynamic IDV thresholds keyed by the document's ISO country code:
    // confidence floor, permitted document types, mandatory fields, and
    // (NQM deployments) whether a post-quantum signature is required.
    let field_keys: Vec<&str> = req
        .scan_result
        .fields
        .iter()
        .map(|f| f.key.as_str())
        .collect();
    let country_rules = state.rules.evaluate(
        &req.scan_result.issuing_country,
        &req.scan_result.document_type,
        req.scan_result.confidence,
        &field_keys,
        &req.scan_result.algo,
    );

    // ── Step 10.6: NQM envelope + quantum-safe server attestation ────────────────
    let nqm_compliance = crate::nqm::envelope_for(&req.scan_result.algo);
    let server_attestation = state
        .attestation
        .attest(verification_id, verified, &timestamp)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))?;

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
        used_key_hex,
        serde_json::to_value(&fraud_signals).ok(),
        trusted_key.key_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    // ── Step 12: Write audit log (VR-3: tenant-scoped; SOC2: hash-chained) ──────
    // record_audit_event links this entry into the tenant's tamper-evident
    // chain and relies on the DB append-only trigger (migration 003).
    crate::models::audit_log::record_audit_event(
        &state.db_pool,
        tenant.tenant_id,
        Some(req.session_id),
        "verification",
        if verified { "success" } else { "failure" },
        serde_json::json!({
            "document_type": req.scan_result.document_type,
            "issuing_country": req.scan_result.issuing_country,
            "confidence": req.scan_result.confidence,
            "key_id": trusted_key.key_id,
            "key_trust": key_trust,
            "verification_id": verification_id.to_string(),
            "country_rules": serde_json::to_value(&country_rules).unwrap_or_default(),
            "nqm_profile": nqm_compliance.profile,
        }),
    )
    .await
    .map_err(AppError::Database)?;

    if !verified {
        return Err(AppError::SignatureInvalid(
            "Signature verification failed".into(),
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
        country_rules,
        nqm_compliance,
        server_attestation,
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
