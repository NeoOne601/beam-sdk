// backend/src/main.rs
// Ajna Verify Backend — Axum-based verification service.
// Provides nonce-protected ML-DSA signature verification,
// audit logging, webhook delivery, and health checks.
//
// Security hardening applied (see README.md §VR-2, §VR-3):
//   - Auth middleware on all routes except /health
//   - Redis token-bucket rate limiting
//   - CORS allowlist (env-var driven, not permissive)
//   - Request body size limit (1 MiB)
//   - Trusted key provider strategy (replaces client-supplied public keys)

use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderName, Method},
    Router,
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod crypto;
mod db;
mod errors;
mod middleware;
mod models;
mod nqm;
mod routes;
mod rules;

use config::AppConfig;
use middleware::key_provider::KeyProvider;

/// Shared application state passed to all route handlers.
pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub redis: redis::Client,
    pub config: AppConfig,
    /// VR-2: Active key provider strategy (selected at startup via KEY_PROVIDER_STRATEGY).
    pub key_provider: Box<dyn KeyProvider>,
    /// Country-Specific Rules Engine: dynamic IDV thresholds by ISO country code.
    pub rules: rules::RulesEngine,
    /// NQM: ML-DSA-65 server attestation key (ephemeral, process lifetime).
    pub attestation: nqm::AttestationSigner,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present (local dev)
    let _ = dotenvy::dotenv();

    // Initialise tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ajna_verify_backend=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;

    // Connect to PostgreSQL via an env-tuned pool. Same binary runs against
    // local Docker Postgres, serverless Neon, or Supabase — only env changes.
    let pool_config = db::pool::PoolConfig::from_env();
    let db_pool = db::pool::connect(&config.database_url, &pool_config).await?;
    tracing::info!(
        max_connections = pool_config.max_connections,
        "Connected to PostgreSQL (pooled)"
    );

    // Connect to Redis
    let redis = redis::Client::open(config.redis_url.as_str())?;
    tracing::info!("Connected to Redis");

    // VR-2: Build the active key provider strategy.
    let key_provider = middleware::key_provider::build_key_provider();

    // VR-3: Build CORS layer from configured origin allowlist.
    let cors_layer = build_cors_layer(&config.cors_origins);

    // Country rules: embedded defaults, or AJNA_COUNTRY_RULES_PATH override.
    let rules = rules::RulesEngine::from_env();

    // NQM: generate the process-lifetime ML-DSA-65 attestation keypair.
    let attestation = nqm::AttestationSigner::new();
    tracing::info!("NQM server attestation key generated (ml-dsa-65)");

    let state = Arc::new(AppState {
        db_pool,
        redis,
        config: config.clone(),
        key_provider,
        rules,
        attestation,
    });

    // VR-3: Authenticated + rate-limited routes.
    let protected_routes = routes::router()
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::rate_limit::rate_limit,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::auth::require_auth,
        ));

    // /health is exempt from auth and rate limiting.
    let app = Router::new()
        .merge(protected_routes)
        .merge(routes::health_router())
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors_layer),
        )
        // VR-3: Limit request bodies to 1 MiB to prevent DoS via large payloads.
        .layer(DefaultBodyLimit::max(1_048_576))
        .with_state(state);

    let bind_addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Ajna Verify Backend listening on {}", bind_addr);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Build a CORS layer from a list of allowed origin strings.
///
/// VR-3: Replaces permissive CORS layer with an explicit allowlist.
/// An empty list disables cross-origin requests entirely (safest default).
fn build_cors_layer(allowed_origins: &[String]) -> CorsLayer {
    if allowed_origins.is_empty() {
        tracing::warn!(
            "CORS_ALLOWED_ORIGINS is not set. Cross-origin requests are disabled. \
             Set this env var for web client access."
        );
        return CorsLayer::new();
    }

    let origins: Vec<axum::http::HeaderValue> = allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    tracing::info!("CORS allowed origins: {:?}", allowed_origins);

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            HeaderName::from_static("x-api-key"),
        ])
        .max_age(std::time::Duration::from_secs(3600))
}
