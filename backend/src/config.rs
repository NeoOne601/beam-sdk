// backend/src/config.rs
// Application configuration loaded from environment variables.

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub port: u16,
    pub nonce_ttl_seconds: u64,
    pub webhook_max_retries: u32,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
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
        })
    }
}
