// backend/src/routes/audit.rs
// GET /v1/audit — Paginated audit log retrieval.
//
// VR-3 (Security): Results are scoped to the authenticated tenant_id.
// TenantContext injected by auth middleware.

use crate::errors::AppError;
use crate::middleware::auth::TenantContext;
use crate::AppState;
use axum::{
    extract::{Extension, Query, State},
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
    State(state): State<Arc<AppState>>,
    Extension(tenant): Extension<TenantContext>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<AuditResponse>, AppError> {
    let limit = query.limit.unwrap_or(100).min(1000);

    // VR-3: Queries are always scoped to the authenticated tenant_id.
    // TODO: Replace with full sqlx query once migrations are applied:
    //   SELECT * FROM audit_logs WHERE tenant_id = $1
    //   ORDER BY created_at DESC LIMIT $2
    tracing::info!(
        tenant_id = %tenant.tenant_id,
        session_id = ?query.session_id,
        from = ?query.from,
        to = ?query.to,
        limit = limit,
        "Audit log query"
    );

    let rows = sqlx::query!(
        r#"
        SELECT id, session_id, event_type, outcome, detail, created_at
        FROM audit_logs
        WHERE tenant_id = $1
        ORDER BY created_at DESC
        LIMIT $2
        "#,
        tenant.tenant_id,
        limit
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    let count_row = sqlx::query!(
        "SELECT COUNT(*) as count FROM audit_logs WHERE tenant_id = $1",
        tenant.tenant_id
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    let total = count_row.count.unwrap_or(0);

    let entries = rows
        .into_iter()
        .map(|r| AuditEntry {
            id: r.id,
            session_id: r.session_id,
            event_type: r.event_type,
            outcome: r.outcome,
            detail: r.detail.unwrap_or(serde_json::json!({})),
            created_at: r.created_at.to_string(),
        })
        .collect();

    Ok(Json(AuditResponse { entries, total }))
}
