// backend/src/routes/health.rs
// GET /health — Service health check.

use crate::AppState;
use axum::{extract::State, Json};
use redis::AsyncCommands;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub db: String,
    pub redis: String,
    pub version: String,
}

pub async fn health_check(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    // Check PostgreSQL
    let db_status = match sqlx::query("SELECT 1").execute(&state.db_pool).await {
        Ok(_) => "ok".to_string(),
        Err(_) => "degraded".to_string(),
    };

    // Check Redis
    let redis_status = match state.redis.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            let ping_result: Result<String, _> = redis::cmd("PING").query_async(&mut conn).await;
            match ping_result {
                Ok(_) => "ok".to_string(),
                Err(_) => "degraded".to_string(),
            }
        }
        Err(_) => "degraded".to_string(),
    };

    let overall = if db_status == "ok" && redis_status == "ok" {
        "ok"
    } else {
        "degraded"
    };

    Json(HealthResponse {
        status: overall.to_string(),
        db: db_status,
        redis: redis_status,
        version: "0.1.0".to_string(),
    })
}
