// backend/src/routes/audit.rs
// GET /v1/audit — Paginated audit log retrieval.

use crate::errors::AppError;
use crate::AppState;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct AuditQuery {
    pub session_id: Option<Uuid>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct AuditEntry {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub event_type: String,
    pub outcome: String,
    pub detail: serde_json::Value,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct AuditResponse {
    pub entries: Vec<AuditEntry>,
    pub total: i64,
}

pub async fn list_audit_logs(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<AuditResponse>, AppError> {
    let limit = query.limit.unwrap_or(100).min(1000);

    // In production, this queries the audit_logs table via sqlx with filters.
    // For initial build, return empty list to satisfy the API contract.
    tracing::info!(
        session_id = ?query.session_id,
        from = ?query.from,
        to = ?query.to,
        limit = limit,
        "Audit log query"
    );

    Ok(Json(AuditResponse {
        entries: Vec::new(),
        total: 0,
    }))
}
