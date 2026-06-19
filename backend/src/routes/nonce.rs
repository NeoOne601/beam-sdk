// backend/src/routes/nonce.rs
// POST /v1/nonce — Generate a single-use nonce for replay prevention.
//
// VR-3 (Security): Redis key is namespaced by tenant_id to prevent cross-tenant
// nonce reuse. TenantContext is extracted from the auth middleware extension.

use crate::errors::AppError;
use crate::middleware::auth::TenantContext;
use crate::AppState;
use axum::{extract::State, Extension, Json};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sqlx;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct NonceRequest {
    pub session_id: Uuid,
}

#[derive(Serialize)]
pub struct NonceResponse {
    pub nonce: String,
    pub expires_at: String,
    pub session_id: Uuid,
    /// VR-3: Echoed back so the client knows which tenant scope was used.
    pub tenant_id: Uuid,
}

pub async fn create_nonce(
    State(state): State<Arc<AppState>>,
    Extension(tenant): Extension<TenantContext>,
    Json(req): Json<NonceRequest>,
) -> Result<Json<NonceResponse>, AppError> {
    // Generate 32-byte random nonce
    let mut nonce_bytes = [0u8; 32];
    getrandom_fill(&mut nonce_bytes);
    let nonce_hex = hex::encode(nonce_bytes);

    // VR-3: Namespace Redis key by tenant_id to prevent cross-tenant nonce collisions.
    let redis_key = format!("beam:nonce:{}:{}", tenant.tenant_id, req.session_id);
    let mut conn = state.redis.get_multiplexed_async_connection().await?;
    conn.set_ex::<_, _, ()>(&redis_key, &nonce_hex, state.config.nonce_ttl_seconds)
        .await?;

    sqlx::query!(
        r#"
        INSERT INTO audit_logs
            (id, tenant_id, session_id, event_type, outcome, detail)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        Uuid::new_v4(),
        tenant.tenant_id,
        req.session_id,
        "nonce_created",
        "success",
        serde_json::json!({
            "nonce_ttl_seconds": state.config.nonce_ttl_seconds,
            "session_id": req.session_id.to_string()
        })
    )
    .execute(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    // Compute expiry timestamp
    let expires_at = time::OffsetDateTime::now_utc()
        + time::Duration::seconds(state.config.nonce_ttl_seconds as i64);
    let expires_str = expires_at
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap_or_else(|_| "unknown".into());

    tracing::info!(
        tenant_id = %tenant.tenant_id,
        session_id = %req.session_id,
        "Nonce issued"
    );

    Ok(Json(NonceResponse {
        nonce: nonce_hex,
        expires_at: expires_str,
        session_id: req.session_id,
        tenant_id: tenant.tenant_id,
    }))
}

/// Fill a buffer with random bytes using rand.
fn getrandom_fill(buf: &mut [u8]) {
    use rand::RngCore;
    rand::thread_rng().fill_bytes(buf);
}
