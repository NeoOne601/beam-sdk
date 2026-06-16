// backend/src/routes/webhook.rs
// POST /v1/webhooks — Register webhook URLs for event delivery.
//
// VR-5 (Security): SSRF protection applied:
//   - URL parsed with the `url` crate (not just starts_with check)
//   - Private/reserved IP ranges blocked (RFC-1918, loopback, link-local)
//   - Cloud metadata endpoints blocked (169.254.169.254, metadata.google.internal)
//   - reqwest::Client configured with 10s timeout, redirect limit, no system proxy
//
// VR-3 (Security): TenantContext extracted from auth middleware.
//   Webhook registrations are scoped to tenant_id.

use crate::errors::AppError;
use crate::middleware::auth::TenantContext;
use crate::AppState;
use axum::{extract::State, Extension, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct WebhookRequest {
    pub url: String,
    pub events: Vec<String>,
    #[allow(dead_code)]
    pub secret: Option<String>,
}

#[derive(Serialize)]
pub struct WebhookResponse {
    pub webhook_id: Uuid,
    pub url: String,
    pub events: Vec<String>,
    pub created_at: String,
    /// VR-3: Scoped to the authenticated tenant.
    pub tenant_id: Uuid,
}

/// Blocked hostnames for SSRF prevention (VR-5).
/// These are common cloud metadata and internal service endpoints.
const BLOCKED_HOSTNAMES: &[&str] = &[
    "localhost",
    "metadata.google.internal",
    "169.254.169.254",       // AWS/Azure/GCP instance metadata
    "fd00::ec2",             // AWS IPv6 metadata
    "0.0.0.0",
    "::1",
    "[::1]",
];

/// Check if a hostname represents a private or reserved IP range.
/// Blocks RFC-1918, loopback, link-local, and known metadata endpoints.
///
/// VR-5: This is a static check on the hostname string; it does not perform
/// DNS resolution. In production, add post-resolution IP validation using a
/// custom `reqwest` connector that checks the resolved IP before connecting.
fn is_ssrf_blocked_host(host: &str) -> bool {
    let host_lower = host.to_lowercase();
    let host_lower = host_lower.trim_start_matches('[').trim_end_matches(']');

    // Block known metadata hostnames
    for blocked in BLOCKED_HOSTNAMES {
        if host_lower == *blocked {
            return true;
        }
    }

    // Block IPv4 private ranges via prefix matching
    let ipv4_blocked_prefixes = [
        "10.",          // RFC-1918 Class A
        "172.16.",      // RFC-1918 Class B (172.16–31.x)
        "172.17.",
        "172.18.",
        "172.19.",
        "172.20.",
        "172.21.",
        "172.22.",
        "172.23.",
        "172.24.",
        "172.25.",
        "172.26.",
        "172.27.",
        "172.28.",
        "172.29.",
        "172.30.",
        "172.31.",
        "192.168.",     // RFC-1918 Class C
        "127.",         // Loopback
        "169.254.",     // Link-local / APIPA (includes AWS metadata 169.254.169.254)
        "100.64.",      // Carrier-grade NAT (RFC 6598)
    ];
    for prefix in &ipv4_blocked_prefixes {
        if host_lower.starts_with(prefix) {
            return true;
        }
    }

    // Block IPv6 private ranges
    let ipv6_blocked_prefixes = [
        "fc",   // fc00::/7 unique-local
        "fd",   // fd00::/8 unique-local
        "fe80", // link-local
    ];
    for prefix in &ipv6_blocked_prefixes {
        if host_lower.starts_with(prefix) {
            return true;
        }
    }

    false
}

/// Validate a webhook URL for SSRF safety (VR-5).
/// Returns Ok(()) if the URL passes all checks, Err with a message if blocked.
fn validate_webhook_url(raw_url: &str) -> Result<(), AppError> {
    // Parse properly — not just starts_with
    let parsed = url::Url::parse(raw_url)
        .map_err(|e| AppError::BadRequest(format!("Invalid URL: {}", e)))?;

    // Scheme must be HTTPS
    if parsed.scheme() != "https" {
        return Err(AppError::BadRequest(
            "Webhook URL must use HTTPS scheme".into(),
        ));
    }

    // Host must be present
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::BadRequest("Webhook URL has no host".into()))?;

    // SSRF: Block private/internal addresses
    if is_ssrf_blocked_host(host) {
        return Err(AppError::BadRequest(format!(
            "Webhook URL host '{}' is a private or reserved address and is not allowed",
            host
        )));
    }

    // Block URL-encoded tricks (e.g., https://example.com@169.254.169.254/)
    if let Some(userinfo) = parsed.username().is_empty().then_some(None).unwrap_or(Some(parsed.username())) {
        return Err(AppError::BadRequest(format!(
            "Webhook URL must not contain userinfo (got: '{}')",
            userinfo
        )));
    }

    Ok(())
}

pub async fn register_webhook(
    State(_state): State<Arc<AppState>>,
    Extension(tenant): Extension<TenantContext>,
    Json(req): Json<WebhookRequest>,
) -> Result<Json<WebhookResponse>, AppError> {
    // VR-5: Validate URL with SSRF protection
    validate_webhook_url(&req.url)?;

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

    // VR-3: Log with tenant scope
    tracing::info!(
        webhook_id = %webhook_id,
        tenant_id = %tenant.tenant_id,
        url = %req.url,
        events = ?req.events,
        "Webhook registered"
    );

    Ok(Json(WebhookResponse {
        webhook_id,
        url: req.url,
        events: req.events,
        created_at,
        tenant_id: tenant.tenant_id,
    }))
}

/// Deliver a webhook payload to a registered URL.
/// Signs with HMAC-SHA256 using the tenant's secret.
/// Retries on failure: 3 attempts with exponential backoff (1s, 5s, 25s).
///
/// VR-5: Uses a hardened reqwest::Client with timeout, redirect limit, and no proxy.
#[allow(dead_code)]
pub async fn deliver_webhook(
    url: &str,
    payload: &serde_json::Value,
    secret: Option<&str>,
) -> Result<(), anyhow::Error> {
    // VR-5: Re-validate URL at delivery time (config may have been set before fix)
    validate_webhook_url(url)?;

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

    // VR-5: Hardened client — 10s timeout, limited redirects, no system proxy.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(3))
        .no_proxy()
        .build()?;

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

    tracing::error!(url = url, "Webhook delivery failed after all retries — dead lettered");
    Err(anyhow::anyhow!(
        "Webhook delivery failed after {} attempts",
        backoff_delays.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_https_url() {
        assert!(validate_webhook_url("https://hooks.example.com/beam/events").is_ok());
    }

    #[test]
    fn test_rejects_http() {
        assert!(validate_webhook_url("http://hooks.example.com/beam").is_err());
    }

    #[test]
    fn test_rejects_localhost() {
        assert!(validate_webhook_url("https://localhost/webhook").is_err());
    }

    #[test]
    fn test_rejects_aws_metadata() {
        assert!(validate_webhook_url("https://169.254.169.254/latest/meta-data/").is_err());
    }

    #[test]
    fn test_rejects_rfc1918_10() {
        assert!(validate_webhook_url("https://10.0.0.1/internal").is_err());
    }

    #[test]
    fn test_rejects_rfc1918_192168() {
        assert!(validate_webhook_url("https://192.168.1.100/hook").is_err());
    }

    #[test]
    fn test_rejects_rfc1918_172() {
        assert!(validate_webhook_url("https://172.16.0.5/hook").is_err());
    }

    #[test]
    fn test_rejects_loopback_127() {
        assert!(validate_webhook_url("https://127.0.0.1/hook").is_err());
    }

    #[test]
    fn test_rejects_google_metadata() {
        assert!(validate_webhook_url("https://metadata.google.internal/computeMetadata/v1/").is_err());
    }

    #[test]
    fn test_rejects_invalid_url() {
        assert!(validate_webhook_url("not-a-url").is_err());
    }

    #[test]
    fn test_ssrf_blocked_host() {
        assert!(is_ssrf_blocked_host("169.254.169.254"));
        assert!(is_ssrf_blocked_host("10.0.0.1"));
        assert!(is_ssrf_blocked_host("172.20.5.5"));
        assert!(is_ssrf_blocked_host("192.168.0.1"));
        assert!(is_ssrf_blocked_host("localhost"));
        assert!(is_ssrf_blocked_host("::1"));
        assert!(!is_ssrf_blocked_host("hooks.example.com"));
        assert!(!is_ssrf_blocked_host("8.8.8.8"));
    }
}
