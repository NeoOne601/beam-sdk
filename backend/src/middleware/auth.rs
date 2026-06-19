// backend/src/middleware/auth.rs
// Authentication middleware — Dual API-key + JWT credential provider.
//
// VR-3 (Security): Enforces tenant authentication on all routes except /health.
// See README.md §VR-3 for the rationale for dual-provider auth.
//
// Credential resolution order:
//   1. X-Api-Key header  → look up tenants.api_key in PostgreSQL (cached in memory).
//   2. Authorization: Bearer <jwt>  → decode and validate JWT; extract tenant_id claim.
//
// Both paths resolve to TenantContext { tenant_id, plan } which is injected
// as a request extension consumed by all downstream handlers.

use crate::errors::AppError;
use crate::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use sqlx::Row;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

/// Authenticated tenant context injected into every authenticated request.
/// Handlers extract this via `Extension<TenantContext>`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub plan: String,
}

/// Extract the raw credential from request headers.
/// Returns (credential_type, credential_value) or None if no credential present.
fn extract_credential(headers: &HeaderMap) -> Option<(&'static str, String)> {
    // Priority 1: X-Api-Key header
    if let Some(api_key) = headers.get("x-api-key") {
        if let Ok(key_str) = api_key.to_str() {
            if !key_str.trim().is_empty() {
                return Some(("api_key", key_str.trim().to_owned()));
            }
        }
    }
    // Priority 2: Authorization: Bearer <jwt>
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                let token = token.trim();
                if !token.is_empty() {
                    return Some(("jwt", token.to_owned()));
                }
            }
        }
    }
    None
}

/// Resolve an API key to a TenantContext by querying the `tenants` table.
async fn resolve_api_key(
    state: &Arc<AppState>,
    api_key: &str,
) -> Result<TenantContext, AppError> {
    // NOTE: In production, add an in-process LRU cache (e.g., moka) keyed by
    // api_key hash to avoid a DB round-trip on every request.
    // The cache TTL should be short (30–60 s) to allow rapid key revocation.
    let row = sqlx::query(
        "SELECT id, plan FROM tenants WHERE api_key = $1 LIMIT 1"
    )
    .bind(api_key)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    match row {
        Some(r) => {
            let id: Uuid = r.try_get("id").map_err(AppError::Database)?;
            let plan: String = r.try_get("plan").map_err(AppError::Database)?;
            Ok(TenantContext {
                tenant_id: id,
                plan,
            })
        },
        None => Err(AppError::Unauthorized("Invalid API key".into())),
    }
}

/// Resolve a JWT bearer token to a TenantContext.
///
/// Production implementation should:
///   - Validate signature against the configured JWKS endpoint.
///   - Check `exp`, `iss`, and `aud` claims.
///   - Extract `tenant_id` from a custom claim.
///
/// This stub validates the token is non-empty and decodes the `tenant_id` claim
/// from an UNSIGNED JWT (base64url payload only). Replace with a full JWT library
/// (e.g., `jsonwebtoken` crate) before production deployment.
#[derive(serde::Deserialize)]
struct JwtClaims {
    tenant_id: String,
    #[allow(dead_code)]
    exp: Option<usize>,
}

async fn resolve_jwt(
    state: &Arc<AppState>,
    token: &str,
) -> Result<TenantContext, AppError> {
    // Read JWT_SECRET from environment. If absent, fall back to stub decode with warning.
    // In production this variable MUST be set. An unset JWT_SECRET is a security risk.
    let secret = std::env::var("JWT_SECRET").unwrap_or_default();

    let tenant_id_str: String = if secret.is_empty() {
        // Dev mode: no signature verification. Log a warning on every call.
        tracing::warn!(
            "JWT_SECRET is not set. JWT signature verification is DISABLED. \
             Set JWT_SECRET=<secret> in production to enforce signature validation."
        );
        // Fallback: decode payload from base64 without verifying signature.
        // This path is identical to the previous stub behaviour.
        let parts: Vec<&str> = token.splitn(3, '.').collect();
        if parts.len() < 2 {
            return Err(AppError::Unauthorized("Malformed JWT".into()));
        }
        let payload_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            parts[1],
        )
        .map_err(|_| AppError::Unauthorized("JWT payload decode failed".into()))?;
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
            .map_err(|_| AppError::Unauthorized("JWT payload not JSON".into()))?;
        payload
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::Unauthorized("JWT missing tenant_id claim".into()))?
            .to_owned()
    } else {
        // Production mode: full HS256 signature verification.
        let mut validation = Validation::new(Algorithm::HS256);
        // Allow tokens without `exp` for service-to-service tokens; set to false
        // to enforce expiry once tokens include `exp` claims.
        validation.validate_exp = false;
        let token_data = decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )
        .map_err(|e| {
            tracing::warn!(error = %e, "JWT signature verification failed");
            AppError::Unauthorized(format!("JWT validation failed: {}", e))
        })?;
        token_data.claims.tenant_id
    };

    let tenant_id = uuid::Uuid::parse_str(&tenant_id_str)
        .map_err(|_| AppError::Unauthorized("JWT tenant_id is not a valid UUID".into()))?;

    let row = sqlx::query!(
        "SELECT plan FROM tenants WHERE id = $1 LIMIT 1",
        tenant_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(AppError::Database)?;

    match row {
        Some(r) => Ok(TenantContext {
            tenant_id,
            plan: r.plan,
        }),
        None => Err(AppError::Unauthorized("Tenant not found".into())),
    }
}

/// Axum middleware function that enforces authentication on every request.
///
/// Injects `TenantContext` as a request extension on success.
/// Returns 401 Unauthorized if no valid credential is present.
///
/// Usage: apply to all routes except `/health` via `Router::route_layer`.
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let credential = extract_credential(req.headers());

    let tenant_ctx = match credential {
        Some(("api_key", key)) => resolve_api_key(&state, &key).await,
        Some(("jwt", token)) => resolve_jwt(&state, &token).await,
        _ => Err(AppError::Unauthorized(
            "Authentication required: provide X-Api-Key or Authorization: Bearer <token>".into(),
        )),
    };

    match tenant_ctx {
        Ok(ctx) => {
            req.extensions_mut().insert(ctx);
            next.run(req).await
        }
        Err(e) => e.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_extract_api_key() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("test-key-123"));
        let result = extract_credential(&headers);
        assert_eq!(result, Some(("api_key", "test-key-123".to_string())));
    }

    #[test]
    fn test_extract_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer eyJhbGciOiJub25lIn0.test.sig"),
        );
        let result = extract_credential(&headers);
        assert_eq!(
            result,
            Some(("jwt", "eyJhbGciOiJub25lIn0.test.sig".to_string()))
        );
    }

    #[test]
    fn test_api_key_takes_priority_over_jwt() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("my-api-key"));
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer some-token"),
        );
        let (cred_type, _) = extract_credential(&headers).unwrap();
        assert_eq!(cred_type, "api_key");
    }

    #[test]
    fn test_no_credential() {
        let headers = HeaderMap::new();
        assert!(extract_credential(&headers).is_none());
    }
}
