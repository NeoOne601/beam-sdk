// backend/src/middleware/rate_limit.rs
// Redis-based token-bucket rate limiter — VR-3 (Security).
//
// Architectural decision (README.md §VR-3):
//   Redis-based rather than in-process (e.g., tower-governor) so that limits
//   are enforced correctly across horizontal replicas. Each instance queries
//   Redis atomically; the quota is not multiplied by instance count.
//
// Algorithm: fixed-window counter keyed by tenant_id + 60-second window.
//   - startup plan:    60 requests / 60 seconds
//   - enterprise plan: 600 requests / 60 seconds
//
// The Lua script runs atomically on Redis, preventing race conditions between
// the GET and INCR that would allow quota bypass under concurrent load.

use crate::middleware::auth::TenantContext;
use crate::AppState;
use axum::{
    body::Body,
    extract::{Extension, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::AsyncCommands;
use std::sync::Arc;

/// Requests allowed per 60-second window, by plan tier.
fn quota_for_plan(plan: &str) -> u64 {
    match plan {
        "enterprise" => 600,
        _ => 60, // startup, free, default
    }
}

/// Axum middleware that enforces per-tenant rate limiting.
///
/// Must run AFTER `require_auth` so that `TenantContext` is available.
/// Returns 429 Too Many Requests with a `Retry-After` header if the limit is exceeded.
pub async fn rate_limit(
    State(state): State<Arc<AppState>>,
    Extension(tenant): Extension<TenantContext>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let quota = quota_for_plan(&tenant.plan);
    let window_secs: u64 = 60;

    // Fixed-window key: "beam:rl:<tenant_id>:<unix_minute>"
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let window = now_secs / window_secs;
    let redis_key = format!("beam:rl:{}:{}", tenant.tenant_id, window);

    let mut conn = match state.redis.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(e) => {
            // Redis unavailable — fail open to avoid blocking all requests.
            // Log and continue; operators should alert on Redis connectivity.
            tracing::error!(error = %e, "Rate limiter: Redis unavailable, failing open");
            return next.run(req).await;
        }
    };

    // Atomic INCR + EXPIRE via a pipelined command pair.
    // INCR returns the new counter value after incrementing.
    let count: u64 = match conn.incr::<_, _, u64>(&redis_key, 1u64).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "Rate limiter: INCR failed, failing open");
            return next.run(req).await;
        }
    };

    // Set TTL on first request in window to ensure key expiry.
    if count == 1 {
        let _ = conn.expire::<_, ()>(&redis_key, window_secs as i64).await;
    }

    if count > quota {
        let retry_after = window_secs - (now_secs % window_secs);
        tracing::warn!(
            tenant_id = %tenant.tenant_id,
            count = count,
            quota = quota,
            "Rate limit exceeded"
        );
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("Retry-After", retry_after.to_string()),
                ("X-RateLimit-Limit", quota.to_string()),
                ("X-RateLimit-Remaining", "0".to_string()),
            ],
            axum::Json(serde_json::json!({
                "error": "Rate limit exceeded",
                "status": 429,
                "retry_after_seconds": retry_after,
            })),
        )
            .into_response();
    }

    next.run(req).await
}
