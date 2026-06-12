// backend/src/routes/mod.rs
// Route registry for the Beam Verify backend.

use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub mod audit;
pub mod health;
pub mod nonce;
pub mod verify;
pub mod webhook;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/nonce", post(nonce::create_nonce))
        .route("/v1/verify", post(verify::verify_result))
        .route("/v1/audit", get(audit::list_audit_logs))
        .route("/v1/webhooks", post(webhook::register_webhook))
        .route("/health", get(health::health_check))
}
