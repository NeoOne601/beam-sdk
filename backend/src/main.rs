// backend/src/main.rs
// Beam Verify Backend — Axum-based verification service.
// Provides nonce-protected ML-DSA signature verification,
// audit logging, webhook delivery, and health checks.

use axum::{middleware, Router};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod crypto;
mod db;
mod errors;
mod models;
mod routes;

use config::AppConfig;

/// Shared application state passed to all route handlers.
pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub redis: redis::Client,
    pub config: AppConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env if present (local dev)
    let _ = dotenvy::dotenv();

    // Initialise tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "beam_verify_backend=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;

    // Connect to PostgreSQL
    let db_pool = sqlx::PgPool::connect(&config.database_url).await?;
    tracing::info!("Connected to PostgreSQL");

    // Connect to Redis
    let redis = redis::Client::open(config.redis_url.as_str())?;
    tracing::info!("Connected to Redis");

    let state = Arc::new(AppState {
        db_pool,
        redis,
        config: config.clone(),
    });

    let app = Router::new()
        .merge(routes::router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let bind_addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Beam Verify Backend listening on {}", bind_addr);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
