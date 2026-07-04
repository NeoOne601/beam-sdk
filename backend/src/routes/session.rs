// NEW FILE — registered in backend/src/routes/mod.rs

use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SessionInitRequest {
    pub client_supported: Vec<String>,
    #[allow(dead_code)]
    pub client_preferred: String,
}

#[derive(Serialize)]
pub struct SessionInitResponse {
    pub session_id: String,
    pub negotiated_algo: String,
    pub server_supported: Vec<String>,
}

#[derive(Serialize)]
pub struct SessionInitError {
    pub error: String,
}

const SERVER_SUPPORTED: &[&str] = &["hybrid-ed25519-ml-dsa-65", "ed25519", "ml-dsa-65"];

pub async fn session_init(
    Json(req): Json<SessionInitRequest>,
) -> Result<Json<SessionInitResponse>, (StatusCode, Json<SessionInitError>)> {
    let negotiated = req
        .client_supported
        .iter()
        .find(|algo| SERVER_SUPPORTED.contains(&algo.as_str()))
        .cloned()
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(SessionInitError {
                    error: "no_common_algorithm".into(),
                }),
            )
        })?;

    Ok(Json(SessionInitResponse {
        session_id: Uuid::new_v4().to_string(),
        negotiated_algo: negotiated,
        server_supported: SERVER_SUPPORTED.iter().map(|s| s.to_string()).collect(),
    }))
}
