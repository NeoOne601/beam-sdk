// backend/src/config.rs
// Application configuration loaded from environment variables.
//
// VR-3 (Security): Added cors_origins (allowlist replaces CorsLayer::permissive)
// and key_provider_strategy (selects trusted key lookup strategy for VR-2).

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub port: u16,
    pub nonce_ttl_seconds: u64,
    pub webhook_max_retries: u32,

    /// VR-3: Comma-separated list of allowed CORS origins.
    /// Example: "https://app.example.com,https://dashboard.example.com"
    /// Empty string (default) disables cross-origin requests.
    /// Set to "*" only in local development — never in production.
    pub cors_origins: Vec<String>,

    /// VR-2: Key provider strategy. One of: "tenant" (default), "device", "model".
    /// See backend/src/middleware/key_provider.rs and README.md §VR-2.
    pub key_provider_strategy: String,

    /// Maximum freshness window for signed result timestamps (seconds).
    /// Results with __timestamp older than this are rejected (VR-1).
    /// Default: 300 seconds (5 minutes) — matches nonce TTL.
    pub result_freshness_window_secs: u64,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let cors_raw = std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_default();
        let cors_origins = if cors_raw.trim().is_empty() {
            Vec::new()
        } else {
            cors_raw
                .split(',')
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        };

        Ok(Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://beam:beam@localhost:5432/beam_verify".into()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".into()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".into())
                .parse()?,
            nonce_ttl_seconds: 300,
            webhook_max_retries: 3,
            cors_origins,
            key_provider_strategy: std::env::var("KEY_PROVIDER_STRATEGY")
                .unwrap_or_else(|_| "tenant".into()),
            result_freshness_window_secs: std::env::var("RESULT_FRESHNESS_WINDOW_SECS")
                .unwrap_or_else(|_| "300".into())
                .parse()
                .unwrap_or(300),
        })
    }
}
