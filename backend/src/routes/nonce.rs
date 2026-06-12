// backend/src/routes/nonce.rs
// POST /v1/nonce — Generate a single-use nonce for replay prevention.

use crate::errors::AppError;
use crate::AppState;
use axum::{extract::State, Json};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
}

pub async fn create_nonce(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NonceRequest>,
) -> Result<Json<NonceResponse>, AppError> {
    // Generate 32-byte random nonce
    let mut nonce_bytes = [0u8; 32];
    getrandom_fill(&mut nonce_bytes);
    let nonce_hex = hex::encode(nonce_bytes);

    // Store in Redis with TTL
    let redis_key = format!("beam:nonce:{}", req.session_id);
    let mut conn = state.redis.get_multiplexed_async_connection().await?;
    conn.set_ex::<_, _, ()>(&redis_key, &nonce_hex, state.config.nonce_ttl_seconds)
        .await?;

    // Compute expiry timestamp
    let expires_at = time::OffsetDateTime::now_utc()
        + time::Duration::seconds(state.config.nonce_ttl_seconds as i64);
    let expires_str = expires_at
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap_or_else(|_| "unknown".into());

    Ok(Json(NonceResponse {
        nonce: nonce_hex,
        expires_at: expires_str,
        session_id: req.session_id,
    }))
}

/// Fill a buffer with random bytes using rand.
fn getrandom_fill(buf: &mut [u8]) {
    use rand::RngCore;
    rand::thread_rng().fill_bytes(buf);
}
