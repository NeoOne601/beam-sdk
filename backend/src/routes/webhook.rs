// backend/src/routes/webhook.rs
// POST /v1/webhooks — Register webhook URLs for event delivery.

use crate::errors::AppError;
use crate::AppState;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct WebhookRequest {
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
}

#[derive(Serialize)]
pub struct WebhookResponse {
    pub webhook_id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    pub created_at: String,
}

pub async fn register_webhook(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<WebhookRequest>,
) -> Result<Json<WebhookResponse>, AppError> {
    // Validate URL format
    if !req.url.starts_with("https://") {
        return Err(AppError::BadRequest("Webhook URL must use HTTPS".into()));
    }

    // Validate event types
    let valid_events = ["verification.complete", "verification.failed"];
    for event in &req.events {
        if !valid_events.contains(&event.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Invalid event type: '{}'. Valid: {:?}",
                event, valid_events
            )));
        }
    }

    let webhook_id = Uuid::new_v4();
    let now = time::OffsetDateTime::now_utc();
    let created_at = now
        .format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap_or_else(|_| "unknown".into());

    // In production, persist to webhook_configs table via sqlx
    tracing::info!(
        webhook_id = %webhook_id,
        url = %req.url,
        events = ?req.events,
        "Webhook registered"
    );

    Ok(Json(WebhookResponse {
        webhook_id,
        url: req.url,
        events: req.events,
        created_at,
    }))
}

/// Deliver a webhook payload to a registered URL.
/// Signs with HMAC-SHA256 using the tenant's secret.
/// Retries on failure: 3 attempts with exponential backoff (1s, 5s, 25s).
#[allow(dead_code)]
pub async fn deliver_webhook(
    url: &str,
    payload: &serde_json::Value,
    secret: Option<&str>,
) -> Result<(), anyhow::Error> {
    let body = serde_json::to_string(payload)?;

    // Sign payload with HMAC-SHA256 if secret is provided
    let signature = if let Some(secret) = secret {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())?;
        mac.update(body.as_bytes());
        let result = mac.finalize();
        Some(format!("sha256={}", hex::encode(result.into_bytes())))
    } else {
        None
    };

    let client = reqwest::Client::new();
    let backoff_delays = [1u64, 5, 25]; // seconds

    for (attempt, delay) in backoff_delays.iter().enumerate() {
        let mut request = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "BeamVerify/0.1.0");

        if let Some(ref sig) = signature {
            request = request.header("X-Beam-Signature", sig.as_str());
        }

        match request.body(body.clone()).send().await {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!(url = url, attempt = attempt + 1, "Webhook delivered");
                return Ok(());
            }
            Ok(resp) => {
                tracing::warn!(
                    url = url,
                    status = %resp.status(),
                    attempt = attempt + 1,
                    "Webhook delivery failed, retrying"
                );
            }
            Err(e) => {
                tracing::warn!(
                    url = url,
                    error = %e,
                    attempt = attempt + 1,
                    "Webhook delivery error, retrying"
                );
            }
        }

        if attempt < backoff_delays.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_secs(*delay)).await;
        }
    }

    // Dead letter: log failure
    tracing::error!(
        url = url,
        "Webhook delivery failed after all retries — dead lettered"
    );
    Err(anyhow::anyhow!(
        "Webhook delivery failed after {} attempts",
        backoff_delays.len()
    ))
}
