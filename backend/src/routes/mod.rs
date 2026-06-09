// backend/src/routes/mod.rs
// Route registry for the Beam Verify backend.

use axum::{Router, routing::{get, post}};
use std::sync::Arc;
use crate::AppState;

pub mod nonce;
pub mod verify;
pub mod audit;
pub mod webhook;
pub mod health;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/nonce", post(nonce::create_nonce))
        .route("/v1/verify", post(verify::verify_result))
        .route("/v1/audit", get(audit::list_audit_logs))
        .route("/v1/webhooks", post(webhook::register_webhook))
        .route("/health", get(health::health_check))
}
