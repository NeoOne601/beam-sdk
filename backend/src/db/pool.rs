// backend/src/db/pool.rs
//
// Env-driven Postgres connection pool. The whole point is decoupling: the same
// binary runs against local Docker Postgres, a serverless Neon branch, or
// Supabase, changing nothing but environment variables.
//
// Serverless Postgres (Neon/Supabase) needs deliberate pool tuning that the
// sqlx default (`PgPool::connect`, 10 conns, no timeouts) gets wrong:
//   - Small max pool: free tiers cap connections hard (Neon ~100, Supabase
//     pooler far less per client). Over-provisioning gets you rejected.
//   - Short idle timeout: serverless scales to zero; holding idle sockets
//     open wastes the quota and racks up compute on Neon.
//   - Acquire timeout: fail fast with a clear error instead of hanging a
//     request when the pool is saturated.
//   - sslmode=require: managed Postgres mandates TLS; we default it on unless
//     the URL already specifies sslmode (local dev uses sslmode=disable).

use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use std::str::FromStr;
use std::time::Duration;

/// Pool tuning, all overridable by env. Defaults are sized for a free
/// serverless tier, not a beefy dedicated instance.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    /// Force sslmode=require when the URL doesn't already set an sslmode.
    pub require_tls: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            // Free-tier safe: a handful of connections per instance. Scale up
            // to enterprise by setting DB_MAX_CONNECTIONS — no code change.
            max_connections: 5,
            min_connections: 0, // scale-to-zero friendly
            acquire_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(30),
            max_lifetime: Duration::from_secs(1800), // 30 min; recycle before provider caps
            require_tls: true,
        }
    }
}

impl PoolConfig {
    /// Load overrides from environment. Unset vars keep the free-tier defaults.
    pub fn from_env() -> Self {
        let d = Self::default();
        Self {
            max_connections: env_parse("DB_MAX_CONNECTIONS", d.max_connections),
            min_connections: env_parse("DB_MIN_CONNECTIONS", d.min_connections),
            acquire_timeout: Duration::from_secs(env_parse(
                "DB_ACQUIRE_TIMEOUT_SECS",
                d.acquire_timeout.as_secs(),
            )),
            idle_timeout: Duration::from_secs(env_parse(
                "DB_IDLE_TIMEOUT_SECS",
                d.idle_timeout.as_secs(),
            )),
            max_lifetime: Duration::from_secs(env_parse(
                "DB_MAX_LIFETIME_SECS",
                d.max_lifetime.as_secs(),
            )),
            // DB_REQUIRE_TLS=false only for local/dev Postgres.
            require_tls: std::env::var("DB_REQUIRE_TLS")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(d.require_tls),
        }
    }
}

/// Build a tuned pool from a database URL and pool config.
pub async fn connect(database_url: &str, config: &PoolConfig) -> Result<PgPool, sqlx::Error> {
    let mut options = PgConnectOptions::from_str(database_url)?;

    // Only force TLS when the URL didn't already pin an sslmode, so an explicit
    // `?sslmode=disable` for local dev is respected.
    if config.require_tls && !url_has_sslmode(database_url) {
        options = options.ssl_mode(sqlx::postgres::PgSslMode::Require);
    }

    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.acquire_timeout)
        .idle_timeout(config.idle_timeout)
        .max_lifetime(config.max_lifetime)
        // test_before_acquire catches connections a serverless provider dropped
        // while idle, so a request never gets a dead socket.
        .test_before_acquire(true)
        .connect_with(options)
        .await
}

fn url_has_sslmode(url: &str) -> bool {
    url.contains("sslmode=")
}

fn env_parse<T: FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_free_tier_safe() {
        let c = PoolConfig::default();
        assert!(c.max_connections <= 10, "free tiers cap connections hard");
        assert_eq!(c.min_connections, 0, "must scale to zero for serverless");
        assert!(c.require_tls, "managed Postgres mandates TLS by default");
    }

    #[test]
    fn env_overrides_are_applied() {
        // Safe to set/remove within one test; keys are unique to this module.
        std::env::set_var("DB_MAX_CONNECTIONS", "42");
        std::env::set_var("DB_REQUIRE_TLS", "false");
        let c = PoolConfig::from_env();
        assert_eq!(c.max_connections, 42);
        assert!(!c.require_tls);
        std::env::remove_var("DB_MAX_CONNECTIONS");
        std::env::remove_var("DB_REQUIRE_TLS");
    }

    #[test]
    fn detects_existing_sslmode_in_url() {
        assert!(url_has_sslmode("postgres://h/db?sslmode=require"));
        assert!(!url_has_sslmode("postgres://h/db"));
    }
}
