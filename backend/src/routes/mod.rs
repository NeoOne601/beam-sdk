// backend/src/routes/mod.rs
// Route registry for the Beam Verify backend.
//
// VR-3 (Security): Routes are split into two groups:
//   router()        — authenticated routes (auth + rate limiting applied in main.rs)
//   health_router() — unauthenticated liveness probe

use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub mod audit;
pub mod health;
pub mod nonce;
pub mod session;
pub mod verify;
pub mod webhook;

/// Authenticated routes — require valid X-Api-Key or Bearer JWT.
/// Auth and rate-limit middleware is applied in main.rs via route_layer.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/nonce", post(nonce::create_nonce))
        .route("/v1/verify", post(verify::verify_result))
        .route("/v1/audit", get(audit::list_audit_logs))
        .route("/v1/webhooks", post(webhook::register_webhook))
        .route("/v1/session/init", post(session::session_init))
}

/// Unauthenticated route for container health probes.
/// Exempt from auth middleware — must not require authentication.
pub fn health_router() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health::health_check))
}
