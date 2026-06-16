// backend/src/middleware/key_provider.rs
// Trusted Public Key Strategy Pattern — VR-2 (Security).
//
// Implements the Strategy Pattern for looking up the trusted ML-DSA public key
// used to verify a scan result signature. Three concrete strategies are provided:
//
//   TenantKeyProvider  — one pre-registered key per tenant (default)
//   DeviceKeyProvider  — per-device key looked up by device_id claim
//   ModelKeyProvider   — per-model-version key looked up by "model_id/model_version"
//
// The active strategy is selected at startup via the KEY_PROVIDER_STRATEGY env var.
// See README.md §VR-2 for full architectural rationale.
//
// All strategies:
//   - Look up ONLY from the trusted_public_keys table (never trust client-supplied keys).
//   - Reject keys where active=FALSE or revoked_at IS NOT NULL.
//   - Return the raw public key bytes for ML-DSA signature verification.

use crate::errors::AppError;
use crate::middleware::auth::TenantContext;
use sqlx::{PgPool, Row};

/// Result of a key lookup — the raw public key bytes ready for ML-DSA verification.
pub struct TrustedKey {
    pub public_key_bytes: Vec<u8>,
    pub key_id: String,
    pub algorithm: String,
}

/// The KeyProvider trait — plug in any key resolution strategy.
/// All implementations MUST only return keys from the trusted_public_keys table.
#[async_trait::async_trait]
pub trait KeyProvider: Send + Sync {
    /// Look up the trusted public key for the given context.
    ///
    /// `tenant`    : authenticated tenant context (from auth middleware)
    /// `key_id`    : the key identifier supplied by the client in the request
    ///               (its meaning depends on the strategy — see module docs)
    async fn get_key(
        &self,
        pool: &PgPool,
        tenant: &TenantContext,
        key_id: &str,
    ) -> Result<TrustedKey, AppError>;
}

// ─── Strategy 1: TenantKeyProvider ───────────────────────────────────────────

/// Looks up a single pre-registered public key for the authenticated tenant.
/// `key_id` in the request is IGNORED — the lookup is by tenant_id only.
///
/// This is the default and simplest strategy, suitable for Phase 1 deployments.
pub struct TenantKeyProvider;

#[async_trait::async_trait]
impl KeyProvider for TenantKeyProvider {
    async fn get_key(
        &self,
        pool: &PgPool,
        tenant: &TenantContext,
        _key_id: &str,
    ) -> Result<TrustedKey, AppError> {
        // Look up by tenant_id. The key_id stored in the DB equals tenant_id::text
        // for this strategy, but we query by tenant_id FK for correctness.
        let row = sqlx::query(
            r#"
            SELECT key_id, public_key, algorithm
            FROM trusted_public_keys
            WHERE tenant_id = $1
              AND active = TRUE
              AND revoked_at IS NULL
            ORDER BY created_at DESC
            LIMIT 1
            "#
        )
        .bind(tenant.tenant_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        row.map(|r| {
            let key_id: String = r.try_get("key_id").unwrap();
            let public_key_bytes: Vec<u8> = r.try_get("public_key").unwrap();
            let algorithm: String = r.try_get("algorithm").unwrap();
            TrustedKey {
                public_key_bytes,
                key_id,
                algorithm,
            }
        })
        .ok_or_else(|| AppError::Unauthorized(
            format!("No registered public key for tenant {}", tenant.tenant_id)
        ))
    }
}

// ─── Strategy 2: DeviceKeyProvider ───────────────────────────────────────────

/// Looks up a per-device public key by `key_id` (which equals the device_id).
/// The device must have been pre-registered in trusted_public_keys by the tenant.
pub struct DeviceKeyProvider;

#[async_trait::async_trait]
impl KeyProvider for DeviceKeyProvider {
    async fn get_key(
        &self,
        pool: &PgPool,
        tenant: &TenantContext,
        key_id: &str,
    ) -> Result<TrustedKey, AppError> {
        if key_id.is_empty() {
            return Err(AppError::BadRequest(
                "key_id (device_id) is required for device key strategy".into(),
            ));
        }

        let row = sqlx::query(
            r#"
            SELECT key_id, public_key, algorithm
            FROM trusted_public_keys
            WHERE tenant_id = $1
              AND key_id    = $2
              AND active    = TRUE
              AND revoked_at IS NULL
            LIMIT 1
            "#
        )
        .bind(tenant.tenant_id)
        .bind(key_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        row.map(|r| {
            let key_id: String = r.try_get("key_id").unwrap();
            let public_key_bytes: Vec<u8> = r.try_get("public_key").unwrap();
            let algorithm: String = r.try_get("algorithm").unwrap();
            TrustedKey {
                public_key_bytes,
                key_id,
                algorithm,
            }
        })
        .ok_or_else(|| AppError::Unauthorized(
            format!("Device key '{}' not registered for tenant {}", key_id, tenant.tenant_id)
        ))
    }
}

// ─── Strategy 3: ModelKeyProvider ────────────────────────────────────────────

/// Looks up a per-model-version public key.
/// `key_id` format: "<model_id>/<model_version>" (e.g., "passport-v2/1.3.0").
/// The model signing key must be pre-registered in trusted_public_keys by the tenant.
pub struct ModelKeyProvider;

#[async_trait::async_trait]
impl KeyProvider for ModelKeyProvider {
    async fn get_key(
        &self,
        pool: &PgPool,
        tenant: &TenantContext,
        key_id: &str,
    ) -> Result<TrustedKey, AppError> {
        if key_id.is_empty() || !key_id.contains('/') {
            return Err(AppError::BadRequest(
                "key_id must be in format 'model_id/model_version' for model key strategy".into(),
            ));
        }

        let row = sqlx::query(
            r#"
            SELECT key_id, public_key, algorithm
            FROM trusted_public_keys
            WHERE tenant_id = $1
              AND key_id    = $2
              AND active    = TRUE
              AND revoked_at IS NULL
            LIMIT 1
            "#
        )
        .bind(tenant.tenant_id)
        .bind(key_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::Database)?;

        row.map(|r| {
            let key_id: String = r.try_get("key_id").unwrap();
            let public_key_bytes: Vec<u8> = r.try_get("public_key").unwrap();
            let algorithm: String = r.try_get("algorithm").unwrap();
            TrustedKey {
                public_key_bytes,
                key_id,
                algorithm,
            }
        })
        .ok_or_else(|| AppError::Unauthorized(
            format!("Model key '{}' not registered for tenant {}", key_id, tenant.tenant_id)
        ))
    }
}

// ─── Strategy factory ─────────────────────────────────────────────────────────

/// Build the active KeyProvider from the KEY_PROVIDER_STRATEGY env var.
///
/// Valid values: "tenant" (default), "device", "model".
/// Unknown values fall back to "tenant" with a warning.
pub fn build_key_provider() -> Box<dyn KeyProvider> {
    let strategy = std::env::var("KEY_PROVIDER_STRATEGY")
        .unwrap_or_else(|_| "tenant".into());
    match strategy.to_lowercase().as_str() {
        "device" => {
            tracing::info!("Key provider strategy: device (per-device pre-registered keys)");
            Box::new(DeviceKeyProvider)
        }
        "model" => {
            tracing::info!("Key provider strategy: model (per-model-version pre-registered keys)");
            Box::new(ModelKeyProvider)
        }
        other => {
            if other != "tenant" {
                tracing::warn!(
                    "Unknown KEY_PROVIDER_STRATEGY '{}'; falling back to 'tenant'",
                    other
                );
            } else {
                tracing::info!("Key provider strategy: tenant (single key per tenant)");
            }
            Box::new(TenantKeyProvider)
        }
    }
}
