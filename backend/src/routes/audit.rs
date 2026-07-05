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

/// GET /v1/audit/verify-chain — SOC2 evidence endpoint.
///
/// Recomputes the tenant's audit hash chain from genesis and reports
/// whether any entry was altered or removed. Uses runtime-checked queries:
/// the chain columns come from migration 003 and are not in the offline
/// sqlx cache.
pub async fn verify_chain(
    State(state): State<Arc<AppState>>,
    Extension(tenant): Extension<TenantContext>,
) -> Result<Json<crate::models::audit_log::ChainReport>, AppError> {
    use crate::models::audit_log::{verify_chain_entries, ChainEntry};
    use sqlx::Row;

    let rows = sqlx::query(
        r#"
        SELECT seq, tenant_id, session_id, event_type, outcome, detail,
               prev_hash, entry_hash
        FROM audit_logs
        WHERE tenant_id = $1
        ORDER BY seq ASC
        "#,
    )
    .bind(tenant.tenant_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    let entries: Vec<ChainEntry> = rows
        .iter()
        .map(|row| ChainEntry {
            seq: row.get("seq"),
            tenant_id: row.get("tenant_id"),
            session_id: row.get("session_id"),
            event_type: row.get("event_type"),
            outcome: row.get("outcome"),
            detail: row
                .get::<Option<serde_json::Value>, _>("detail")
                .unwrap_or_else(|| serde_json::json!({})),
            prev_hash: row.get("prev_hash"),
            entry_hash: row.get("entry_hash"),
        })
        .collect();

    Ok(Json(verify_chain_entries(&entries)))
}
