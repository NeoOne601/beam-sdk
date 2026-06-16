// backend/src/middleware/mod.rs
// Axum middleware registry for the Beam Verify backend.
//
// VR-3 (Security): Provides authentication/authorization and rate-limiting layers.
// See README.md §VR-3 for architectural rationale.

pub mod auth;
pub mod key_provider;
pub mod rate_limit;
