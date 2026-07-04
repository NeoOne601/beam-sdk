# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Beam SDK: cross-platform native document-scanning SDK with post-quantum cryptographic result
integrity. Rust core (`core/`) does all business logic (quality gates, session state machine,
PQC signing, FFI boundary). C++ sits only at the ML runtime boundary (TFLite/CoreML/ONNX Runtime)
for zero-copy tensor delivery. Swift/Kotlin are thin camera adapters with no business logic. A
separate Rust Axum service (`backend/`) verifies signed scan results server-side.

Full architecture, sequence diagrams, and the security remediation log (VR-1..VR-6) live in
[README.md](README.md) — read it before making non-trivial changes; it is the primary design doc,
not just an intro.

## Workspace layout

Cargo workspace members: `core`, `backend`, `core/fuzz`, `crates/beam-crypto`.

- `core/` — `beam-core`, the on-device SDK core. Built as `staticlib` + `cdylib` + `rlib` and
  cross-compiled to Android/iOS/WASM. Depends on `beam-crypto`.
- `crates/beam-crypto/` — standalone signing crate, no knowledge of scan/session logic. Exposes
  the `BeamSigner` trait + `SignerRegistry` (crypto agility registry, ADR-001) and a process-wide
  `OnceLock<RwLock<SignerRegistry>>` via `global_registry()` / `init_registry()`. Concrete signers
  live under `signers/`: `EdDsaSigner` (default), `MlDsaSigner` (FIPS 204 Dilithium-3, behind the
  `pqc` feature), `HybridSigner`, `EcdsaSigner` (stub). `jws.rs` produces dual-envelope JWS output
  for classical algorithms.
- `backend/` — `beam-verify-backend`, an Axum service that verifies signed `ScanResult`s
  server-side, issues nonces, and logs audit trail. Not part of the on-device SDK.
- `platform/{android,ios,wasm}` — C++/Swift/Kotlin bridges consumed by `build/CMakeLists.txt`.
  These call into `beam-core` only through `include/beam_ffi.h`.

## Commands

### Rust core (`core/`)
```bash
cd core && cargo test --release                       # all tests
cargo test --release -p beam-core --test quality_gate_tests            # one test file
cargo test --release -p beam-core --test quality_gate_tests -- exact_name  # one test
cargo fmt --check -p beam-core && cargo clippy -p beam-core -- -D warnings
cargo bench -p beam-core --bench pipeline_bench -- --test    # dry-run bench (what CI runs)
cargo bench -p beam-core                                      # full Criterion run
```
Test files: `tests/quality_gate_tests.rs`, `tests/session_state_tests.rs`, `tests/crypto_pqc_tests.rs`.

### beam-crypto
```bash
cargo test -p beam-crypto                     # default features (no PQC)
cargo test -p beam-crypto --features pqc      # include ML-DSA signer tests
```

### Backend (`backend/`)
Needs Postgres + Redis — `docker-compose up -d` from `backend/` starts both with the same
creds `main.rs`/`config.rs` default to (`postgres://beam:beam@localhost:5432/beam_verify`,
`redis://127.0.0.1:6379`).
```bash
cd backend
psql "$DATABASE_URL" -f src/db/migrations/001_initial.sql
psql "$DATABASE_URL" -f src/db/migrations/002_trusted_keys.sql
cargo test --release -p beam-verify-backend
cargo fmt --check -p beam-verify-backend && cargo clippy -p beam-verify-backend -- -D warnings -A unused
```
`.sqlx/` holds cached query metadata for `sqlx`'s compile-time query checking — regenerate with
`cargo sqlx prepare` (from `backend/`) after changing any `sqlx::query!`/`query_as!` call, with
`DATABASE_URL` pointing at a live, migrated database.

### C++ FFI integration tests
```bash
# after `cargo build --release` in core/ for the host target:
clang++ -O2 tests/ffi_integration_tests.cpp -I include/ -L core/target/release \
    -lbeam_core -lpthread -ldl -o ffi_tests
./ffi_tests
valgrind --leak-check=full --error-exitcode=1 ./ffi_tests   # expect 0 bytes definitely lost
```

### Mobile/WASM cross-compilation
Full toolchain setup (NDK/Xcode/Emscripten env vars) is in README.md "Build System" — don't
re-derive it, follow that section verbatim. Quick reference:
```bash
cd core && cargo build --release --target aarch64-linux-android   # Android arm64
cd core && cargo build --release --target aarch64-apple-ios       # iOS device
cd core && cargo build --release --target wasm32-unknown-emscripten  # WASM
```

## Non-obvious invariants (grep-enforced by CI, do not regress silently)

`.github/workflows/ci.yml`'s `security-checks` job greps for these patterns on every push. If you
touch the related files, keep the invariant or update the CI check deliberately — don't just make
the grep pass:

- **`core/src/pipeline.rs`**: `AcceptedForInference` must be returned from exactly one place, only
  when `gate_reached == Gate::Accepted`. C++ must never see a rejected frame.
- **`core/src/result.rs`**: `canonical_bytes()` must include `__nonce`, `__session_id`,
  `__timestamp` (VR-1 nonce-binding — prevents replaying a signed result into a new session).
- **`backend/src/routes/verify.rs`**: must never trust a client-supplied `pqc_public_key` (VR-2 —
  keys are resolved server-side via `KeyProvider`, strategy selected by `KEY_PROVIDER_STRATEGY`
  env var: `tenant` (default) / `device` / `model`).
- **`backend/src/`**: no `CorsLayer::permissive()` anywhere (VR-3 — CORS is an explicit allowlist
  built from `CORS_ALLOWED_ORIGINS` in `main.rs::build_cors_layer`).
- **`backend/src/routes/webhook.rs`**: must call `validate_webhook_url` (VR-5 SSRF protection —
  blocks RFC-1918/loopback/link-local/cloud-metadata hosts).
- CI also fails on any `fips certified|fips validated|fips compliant|never leaves the enclave` in
  `docs/`, and on any `cargo audit` finding (no `--ignore` suppressions allowed).

## Backend request flow

`main.rs` wires two router groups: `routes::router()` (auth + Redis token-bucket rate limiting
applied via `route_layer`, in that order — see `middleware/auth.rs`, `middleware/rate_limit.rs`)
and `routes::health_router()` (`/health`, deliberately exempt from both). Auth accepts `X-Api-Key`
first, falls back to `Authorization: Bearer <jwt>` (HS256 via `jsonwebtoken`); both resolve to a
tenant context used to scope every query. Routes live in `backend/src/routes/`: `nonce`, `verify`,
`audit`, `webhook`, `session` (algorithm negotiation endpoint, `/v1/session/init`).

## Key source files

| File | Contents |
|---|---|
| `core/src/quality.rs` | Quality gates (blur/exposure/motion/boundary), thresholds, adaptive relaxation |
| `core/src/session.rs` | Session state machine, timeout, adaptive gate limit |
| `core/src/result.rs` | `ScanResult`, `canonical_bytes()` deterministic encoding |
| `core/src/crypto.rs` | mlock-protected key handling, zero-on-drop (SDK-side, pre-dates `beam-crypto`) |
| `core/src/pipeline.rs` | `FramePipeline` orchestrator — the accept/reject invariant lives here |
| `core/src/ffi.rs` | All `#[no_mangle]` C exports; every `unsafe` block has a `Safety:` comment |
| `crates/beam-crypto/src/signer.rs` | `BeamSigner` trait + `SignerRegistry` |
| `include/beam_ffi.h` | Canonical C header consumed by all three platform bridges |
| `backend/src/config.rs` | Env-var driven `AppConfig` — see for every backend env var and its default |
