# Ajna SDK — ADR-001 Crypto Agility Layer
# Antigravity Multi-Model Prompt System — v3 (Clean First Pass)
#
# WHAT CHANGED FROM v2
# ─────────────────────
# Fix 1 (Prompt 2): Removed #[serde(default)] — ScanResult does not derive Deserialize.
#         Serialization is manual via serde_json::json!() in ffi.rs.
# Fix 2 (Prompt 2): Added File 9 — all 6 existing ScanResult construction sites
#         (field_parser.rs, 3 in crypto_pqc_tests.rs, session_state_tests.rs,
#         pipeline_bench.rs, crypto_bench.rs) now updated with the 3 new fields.
#         Without this: every cargo test and cargo bench fails immediately.
# Fix 3 (Prompt 3): Added base64 = "0.21" to core/Cargo.toml dependencies.
#         Without this: base64 encoding in ffi.rs fails to compile.
# Fix 4 (Prompt 3): Added ajna_session_get_result_json() update.
#         The json!() block in ffi.rs:387-398 must include the 3 new fields.
# Fix 5 (Prompt 1): Removed once_cell = "1" from ajna-crypto/Cargo.toml.
#         std::sync::OnceLock is used (stable since Rust 1.70). No external crate needed.
#
# HOW TO RUN
# ─────────────────────────────────────────────────────────────────────────────
# Paste each prompt into Antigravity after setting the specified model.
# Each prompt ends with an explicit ✅ line telling you which model to set next.
# You are the router — the model does not switch itself.
#
# Sequence:
#   PROMPT 1 → Claude Opus 4.6 (Thinking)    — trait + FFI scaffold + workspace fix
#   PROMPT 2 → Claude Sonnet 4.6 (Thinking)  — implementations + endpoint + envelope + fixups
#   PROMPT 3 → Claude Opus 4.6 (Thinking)    — ffi.rs registry wiring + review
#   PROMPT 4 → Gemini 3.1 Pro (High)         — build, test, git, CI loop to green
# ─────────────────────────────────────────────────────────────────────────────


═══════════════════════════════════════════════════════════════
PROMPT 1 OF 4
SET ANTIGRAVITY MODEL TO: Claude Opus 4.6 (Thinking)
PHASE: Trait design, FFI scaffold, workspace registration
═══════════════════════════════════════════════════════════════

You are a senior Rust systems engineer implementing ADR-001 for the Ajna SDK.

## Verified codebase state (read before writing anything)
- Workspace Cargo.toml (repo root): members = ["core", "backend", "core/fuzz"]
  crates/ajna-crypto does NOT exist yet — you are creating it.
- Signing currently lives in: core/src/ffi.rs, function ajna_session_push_result()
  which calls crate::crypto::PqcSigner::generate() at lines 322-330.
- C++ bridges (tflite_bridge.cpp, coreml_bridge.mm) call ajna_session_push_result()
  with include_pqc_sig=true — they do NOT call any signing function directly.
- ajna_ffi.h does NOT need a ajna_sign() declaration.
- Architecture decision: signing is abstracted via a SignerRegistry in the new
  ajna-crypto crate. core/src/ffi.rs will call through the registry.
  The C++ bridges remain unchanged. ajna_ffi.h remains unchanged.

## Your task — write exactly these four files, nothing else

### File 1: crates/ajna-crypto/src/signer.rs
Define the AjnaSigner trait and SignerRegistry.

Requirements:
- `pub trait AjnaSigner: Send + Sync` with exactly these four methods:
    fn algorithm_id(&self) -> &'static str;
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError>;
    fn verify(&self, payload: &[u8], sig: &[u8]) -> Result<bool, SignerError>;
    fn public_key_bytes(&self) -> Vec<u8>;
- `pub struct SignerRegistry` containing:
    signers: HashMap<String, Arc<dyn AjnaSigner>>
    preferred_order: Vec<String>
- `impl SignerRegistry` with these methods and no others:
    pub fn new() -> Self
    pub fn register(&mut self, signer: impl AjnaSigner + 'static) -> &mut Self
      — inserts into signers map keyed by signer.algorithm_id()
      — appends algorithm_id to preferred_order
      — returns &mut Self for chaining
    pub fn select(&self, algo_id: &str) -> Option<Arc<dyn AjnaSigner>>
      — returns None (never panics) if algo_id not found
    pub fn preferred(&self) -> Option<Arc<dyn AjnaSigner>>
      — returns the first signer in preferred_order that is present in signers, or None
      — if preferred_order is empty, returns None (no panic)
    pub fn supported_algorithms(&self) -> Vec<String>
      — returns preferred_order.clone()
- `pub enum SignerError` with variants:
    SigningFailed(String)
    VerificationFailed(String)
    NotImplemented(String)
    InvalidKey(String)
- Derive thiserror::Error and std::fmt::Display on SignerError with #[error("...")] on each variant
- Imports needed: std::collections::HashMap, std::sync::Arc, thiserror::Error

### File 2: crates/ajna-crypto/src/lib.rs
- pub mod signer;
- pub mod signers;
- pub use signer::{AjnaSigner, SignerError, SignerRegistry};
- A global SignerRegistry instance using std::sync::OnceLock (no external once_cell crate):
    use std::sync::{OnceLock, RwLock};
    static GLOBAL_REGISTRY: OnceLock<RwLock<SignerRegistry>> = OnceLock::new();
    pub fn global_registry() -> &'static RwLock<SignerRegistry>
      — calls GLOBAL_REGISTRY.get_or_init(|| RwLock::new(SignerRegistry::new()))
    pub fn init_registry(registry: SignerRegistry)
      — calls GLOBAL_REGISTRY.set(RwLock::new(registry))
      — if already set: panics with a clear message ("init_registry called twice")
- Do NOT write a ajna_sign() C extern function here.
  The entry point into signing from ffi.rs will use global_registry() directly.
- Re-export signers module contents: pub use signers::*;

### File 3: crates/ajna-crypto/Cargo.toml
[package]
name = "ajna-crypto"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["rlib"]

[dependencies]
thiserror = "1"
ed25519-dalek = { version = "2", features = ["rand_core"] }
rand = "0.8"
pqcrypto-dilithium = { version = "0.5", optional = true }
pqcrypto-traits = { version = "0.3", optional = true }

[features]
default = []
pqc = ["pqcrypto-dilithium", "pqcrypto-traits"]

NOTE: Do NOT include once_cell. OnceLock is in std since Rust 1.70.

### File 4: Cargo.toml (workspace root — MODIFY existing file)
Add "crates/ajna-crypto" to the members list.
The complete file must be:

[workspace]
members = ["core", "backend", "core/fuzz", "crates/ajna-crypto"]
resolver = "2"

## Output format — STRICT
Output each file as:
/// FILE: <relative/path/from/repo/root>
<complete file content — compilable, no ellipsis, no truncation>

Then output exactly this line and nothing after it:
✅ Phase 1 complete — switch model to Claude Sonnet 4.6 (Thinking) and paste Prompt 2


═══════════════════════════════════════════════════════════════
PROMPT 2 OF 4
SET ANTIGRAVITY MODEL TO: Claude Sonnet 4.6 (Thinking)
PHASE: Signer implementations, session endpoint, payload envelope, construction site fixups
═══════════════════════════════════════════════════════════════

You are a senior Rust engineer. Phase 1 has written:
- crates/ajna-crypto/src/signer.rs   (AjnaSigner trait, SignerRegistry, SignerError)
- crates/ajna-crypto/src/lib.rs      (global_registry(), init_registry(), pub mods)
- crates/ajna-crypto/Cargo.toml
- Workspace Cargo.toml updated to include crates/ajna-crypto

## Verified codebase state (read before writing anything)
- Payload struct to modify: core/src/result.rs — struct ScanResult
  Currently has fields: fields, raw_mrz, document_type, issuing_country,
  confidence, pqc_signature (Vec<u8>), pqc_public_key (Vec<u8>),
  nonce (Option<String>), session_id (Option<String>), timestamp_iso (Option<String>)
  ScanResult derives ONLY: Debug, Clone. It does NOT derive Serialize or Deserialize.
  Do NOT add serde derives. Do NOT add #[serde(default)].
  Serialization is handled manually in core/src/ffi.rs via serde_json::json!() macro.
- Session route does not exist yet. Existing routes in backend/src/routes/mod.rs:
  nonce, verify, audit, webhook, health
- uuid = { version = "1", features = ["v4", "serde"] } already exists in backend/Cargo.toml
- All signer files go under: crates/ajna-crypto/src/signers/

## CRITICAL: naming rules
- The Ed25519 signer file is named ed25519_signer.rs (NOT ed25519.rs)
- mod.rs declares pub mod ed25519_signer (NOT pub mod ed25519)
- A mismatch causes an immediate compile error

## Your task — write exactly these files listed below, nothing else

### File 1: crates/ajna-crypto/src/signers/ed25519_signer.rs
- `pub struct EdDsaSigner { signing_key: ed25519_dalek::SigningKey }`
- `impl EdDsaSigner`: pub fn new() -> Self
    use rand::rngs::OsRng;
    Self { signing_key: ed25519_dalek::SigningKey::generate(&mut OsRng) }
- `impl AjnaSigner for EdDsaSigner`:
    algorithm_id() -> "ed25519"
    sign():
      use ed25519_dalek::Signer;
      let sig = self.signing_key.sign(payload);
      Ok(sig.to_bytes().to_vec())
    verify():
      use ed25519_dalek::{Signature, Verifier};
      let vk = self.signing_key.verifying_key();
      let sig = Signature::from_slice(sig)
        .map_err(|e| SignerError::VerificationFailed(e.to_string()))?;
      vk.verify(payload, &sig)
        .map(|_| true)
        .map_err(|e| SignerError::VerificationFailed(e.to_string()))
    public_key_bytes(): self.signing_key.verifying_key().to_bytes().to_vec()
- Imports: use crate::signer::{AjnaSigner, SignerError};

### File 2: crates/ajna-crypto/src/signers/ml_dsa.rs
- Gate entire file: wrap ALL content (struct, impls, use statements) in
  #[cfg(feature = "pqc")] blocks so the file compiles as empty without the feature.
- `pub struct MlDsaSigner { public_key: pqcrypto_dilithium::dilithium3::PublicKey,
                              secret_key: pqcrypto_dilithium::dilithium3::SecretKey }`
- `impl MlDsaSigner`: pub fn new() -> Self
    let (pk, sk) = pqcrypto_dilithium::dilithium3::keypair();
    Self { public_key: pk, secret_key: sk }
- `impl AjnaSigner for MlDsaSigner`:
    algorithm_id() -> "ml-dsa-65"
    sign():
      use pqcrypto_dilithium::dilithium3::detached_sign;
      use pqcrypto_traits::sign::DetachedSignature;
      let sig = detached_sign(payload, &self.secret_key);
      Ok(sig.as_bytes().to_vec())
    verify():
      use pqcrypto_dilithium::dilithium3::{verify_detached_signature, DetachedSignature};
      use pqcrypto_traits::sign::DetachedSignature as DetachedSignatureTrait;
      let sig_obj = DetachedSignature::from_bytes(sig)
        .map_err(|e| SignerError::VerificationFailed(format!("{:?}", e)))?;
      verify_detached_signature(&sig_obj, payload, &self.public_key)
        .map(|_| true)
        .map_err(|e| SignerError::VerificationFailed(format!("{:?}", e)))
    public_key_bytes():
      use pqcrypto_traits::sign::PublicKey;
      self.public_key.as_bytes().to_vec()
- Imports: use crate::signer::{AjnaSigner, SignerError};

### File 3: crates/ajna-crypto/src/signers/ecdsa_stub.rs
- `pub struct EcdsaSigner;`
- `impl AjnaSigner for EcdsaSigner`:
    algorithm_id() -> "ecdsa-p256"
    sign() -> Err(SignerError::NotImplemented("EcdsaSigner ships in Phase 2".into()))
    verify() -> Err(SignerError::NotImplemented("EcdsaSigner ships in Phase 2".into()))
    public_key_bytes() -> vec![]
- Imports: use crate::signer::{AjnaSigner, SignerError};

### File 4: crates/ajna-crypto/src/signers/hybrid_stub.rs
- `pub struct HybridSigner;`
- `impl AjnaSigner for HybridSigner`:
    algorithm_id() -> "hybrid-ed25519-ml-dsa-65"
    sign() -> Err(SignerError::NotImplemented("HybridSigner ships in Phase 2".into()))
    verify() -> Err(SignerError::NotImplemented("HybridSigner ships in Phase 2".into()))
    public_key_bytes() -> vec![]
- Imports: use crate::signer::{AjnaSigner, SignerError};

### File 5: crates/ajna-crypto/src/signers/mod.rs
pub mod ed25519_signer;
pub mod ml_dsa;
pub mod ecdsa_stub;
pub mod hybrid_stub;

pub use ed25519_signer::EdDsaSigner;
#[cfg(feature = "pqc")]
pub use ml_dsa::MlDsaSigner;
pub use ecdsa_stub::EcdsaSigner;
pub use hybrid_stub::HybridSigner;

### File 6: backend/src/routes/session.rs  (NEW FILE)
Line 1 must be: // NEW FILE — registered in backend/src/routes/mod.rs

Write a POST /v1/session/init Axum handler:

use axum::{http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SessionInitRequest {
    pub client_supported: Vec<String>,
    pub client_preferred: String,
}

#[derive(Serialize)]
pub struct SessionInitResponse {
    pub session_id: String,
    pub negotiated_algo: String,
    pub server_supported: Vec<String>,
}

#[derive(Serialize)]
pub struct SessionInitError {
    pub error: String,
}

const SERVER_SUPPORTED: &[&str] = &["ed25519", "ml-dsa-65"];

pub async fn session_init(
    Json(req): Json<SessionInitRequest>,
) -> Result<Json<SessionInitResponse>, (StatusCode, Json<SessionInitError>)> {
    let negotiated = req.client_supported
        .iter()
        .find(|algo| SERVER_SUPPORTED.contains(&algo.as_str()))
        .cloned()
        .ok_or_else(|| (
            StatusCode::BAD_REQUEST,
            Json(SessionInitError { error: "no_common_algorithm".into() }),
        ))?;

    Ok(Json(SessionInitResponse {
        session_id: Uuid::new_v4().to_string(),
        negotiated_algo: negotiated,
        server_supported: SERVER_SUPPORTED.iter().map(|s| s.to_string()).collect(),
    }))
}

### File 7: backend/src/routes/mod.rs  (MODIFY existing file)
Add `pub mod session;` to the module list.
Add `.route("/v1/session/init", post(session::session_init))` to the router() function.

The complete file must be:

// backend/src/routes/mod.rs
// Route registry for the Ajna Verify backend.
//
// VR-3 (Security): Routes are split into two groups:
//   router()        — authenticated routes (auth + rate limiting applied in main.rs)
//   health_router() — unauthenticated liveness probe

use crate::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub mod audit;
pub mod health;
pub mod nonce;
pub mod session;
pub mod verify;
pub mod webhook;

/// Authenticated routes — require valid X-Api-Key or Bearer JWT.
/// Auth and rate-limit middleware is applied in main.rs via route_layer.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/v1/nonce", post(nonce::create_nonce))
        .route("/v1/verify", post(verify::verify_result))
        .route("/v1/audit", get(audit::list_audit_logs))
        .route("/v1/webhooks", post(webhook::register_webhook))
        .route("/v1/session/init", post(session::session_init))
}

/// Unauthenticated route for container health probes.
/// Exempt from auth middleware — must not require authentication.
pub fn health_router() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health::health_check))
}

### File 8: core/src/result.rs  (MODIFY existing file)
Add three new fields to ScanResult, AFTER the existing timestamp_iso field:

    /// Signing algorithm identifier (e.g. "ed25519", "ml-dsa-65").
    /// Empty string when no signing was performed.
    pub algo: String,
    /// Ajna SDK version string. Always "2.0" for this release.
    pub ajna_version: String,
    /// Base64-encoded public key bytes from the signer.
    /// Empty string when no signing was performed.
    pub public_key: String,

Do NOT add #[serde(default)], #[derive(Serialize)], or #[derive(Deserialize)].
The struct stays as #[derive(Debug, Clone)]. Serialization is manual in ffi.rs.

Output the COMPLETE modified file — not just the added lines.

### File 9: Fix ALL existing ScanResult construction sites  (CRITICAL)
Adding fields to a struct in Rust breaks every existing construction site.
You MUST update the following 5 files to add the 3 new fields to every
ScanResult { ... } literal. Use these default values in every construction:
    algo: String::new(),
    ajna_version: String::from("2.0"),
    public_key: String::new(),

Files containing ScanResult literals (output EACH as a complete file):

1. core/src/field_parser.rs — function to_scan_result() at line ~150
   Add the 3 fields after timestamp_iso: None,

2. core/tests/crypto_pqc_tests.rs — 3 ScanResult literals at lines ~91, ~123, ~138
   Add the 3 fields after timestamp_iso: None, in EACH of the 3 literals

3. core/tests/session_state_tests.rs — function default_result() at line ~11
   Add the 3 fields after timestamp_iso: None,

4. core/benches/crypto_bench.rs — ScanResult literal at line ~57
   Add the 3 fields after timestamp_iso: None,

5. core/benches/pipeline_bench.rs — ScanResult literal at line ~122
   Add the 3 fields after timestamp_iso: None,

IMPORTANT: Output each of these 5 files in COMPLETE form. Do NOT truncate.
Do NOT use "..." or "// rest unchanged". Every line of every file must be present.
These files are large — output them fully anyway. A truncated file is worse than
no file because it deletes the truncated content.

NOTE: core/src/ffi.rs also has a ScanResult literal at line ~309 — but that file
will be fully rewritten in Prompt 3. Do NOT output ffi.rs in this prompt.

## Output format — STRICT
Output each file as:
/// FILE: <relative/path/from/repo/root>
<complete file content — compilable, no ellipsis, no truncation>

Then output exactly this line and nothing after it:
✅ Phase 2 complete — switch model to Claude Opus 4.6 (Thinking) and paste Prompt 3


═══════════════════════════════════════════════════════════════
PROMPT 3 OF 4
SET ANTIGRAVITY MODEL TO: Claude Opus 4.6 (Thinking)
PHASE: Wire SignerRegistry into ffi.rs + cross-file correctness review
═══════════════════════════════════════════════════════════════

You are a senior Rust systems engineer. Phases 1 and 2 have completed.

## Verified codebase state — architecture clarification (critical)
The C++ bridges (platform/android/tflite_bridge.cpp, platform/ios/coreml_bridge.mm)
do NOT call any signing function directly. They call:
    ajna_session_push_result(..., include_pqc_sig: true, ...)

The actual signing call is inside core/src/ffi.rs, in the ajna_session_push_result()
function, at lines 322-330, which currently calls:
    crate::crypto::PqcSigner::generate(crate::crypto::MlDsaLevel::Level3)

This is the ONLY signing call site. The C++ bridges are NOT modified.
Do not touch tflite_bridge.cpp, coreml_bridge.mm, or ajna_ffi.h.

## Your task — three parts

### Part A: Wire SignerRegistry into core/src/ffi.rs

In core/src/ffi.rs, locate the ajna_session_push_result() function.
Find the hardwired signing block at lines ~322-330:

    if include_pqc_sig && session.config.pqc_sign_result {
        if let Ok(signer) = crate::crypto::PqcSigner::generate(crate::crypto::MlDsaLevel::Level3) {
            let canonical = result.canonical_bytes();
            if let Ok(sig) = signer.sign(&canonical) {
                result.pqc_signature = sig;
                result.pqc_public_key = signer.public_key_bytes().to_vec();
            }
        }
    }

Replace this block with a registry dispatch:

    if include_pqc_sig && session.config.pqc_sign_result {
        if let Ok(registry) = ajna_crypto::global_registry().read() {
            if let Some(signer) = registry.preferred() {
                let canonical = result.canonical_bytes();
                if let Ok(sig) = signer.sign(&canonical) {
                    result.pqc_signature = sig;
                    result.pqc_public_key = signer.public_key_bytes();
                    result.algo = signer.algorithm_id().to_string();
                    result.public_key = base64::engine::general_purpose::STANDARD
                        .encode(&signer.public_key_bytes());
                }
            }
        }
    }

Also add to the imports at the top of ffi.rs:
    use base64::Engine;

Also update the ScanResult literal at line ~309 to include the 3 new fields:
    algo: String::new(),
    ajna_version: String::from("2.0"),
    public_key: String::new(),

Also update the ajna_session_get_result_json() function (~lines 387-398).
The json!() block currently serializes:
    "pqc_signature_hex": hex::encode(&result.pqc_signature),
    "pqc_public_key_hex": hex::encode(&result.pqc_public_key)
Add these 3 new fields to the json!() block:
    "algo": result.algo,
    "ajna_version": result.ajna_version,
    "public_key": result.public_key,

Output the COMPLETE modified core/src/ffi.rs — every line, no truncation.

### Part B: Add dependencies to core/Cargo.toml

Add these two dependencies to core/Cargo.toml [dependencies] section:
    ajna-crypto = { path = "../crates/ajna-crypto" }
    base64 = "0.21"

Output the COMPLETE modified core/Cargo.toml — every line, no truncation.

### Part C: Cross-file correctness review

Check each of the following and output the corrected file in full if a fix is needed,
or "✓ [filename] — no issues" if it passes:

1. crates/ajna-crypto/src/signer.rs
   — SignerRegistry::select() returns None for unknown algo_id (no unwrap, no panic)
   — SignerRegistry::preferred() returns None if preferred_order is empty (no panic)
   — SignerError variants all have #[error("...")] attributes

2. crates/ajna-crypto/src/lib.rs
   — global_registry() is safe to call from multiple threads (OnceLock + RwLock)
   — init_registry() panics with a clear message if called twice
   — No once_cell crate import — uses only std::sync::OnceLock

3. crates/ajna-crypto/src/signers/ed25519_signer.rs
   — File name matches mod.rs declaration (ed25519_signer, NOT ed25519)
   — ed25519_dalek::Signer trait is imported for .sign() method

4. crates/ajna-crypto/src/signers/ml_dsa.rs
   — Entire file content is gated by #[cfg(feature = "pqc")]
   — pqcrypto_traits imports match the trait methods used (DetachedSignature, PublicKey)
   — Uses detached_sign / verify_detached_signature (NOT sign/open which produce SignedMessage)

5. backend/src/routes/session.rs
   — Handler compiles with axum 0.7 (Json extractor, Result return type)
   — no_common_algorithm returns StatusCode::BAD_REQUEST (400, not 500)

6. core/src/result.rs
   — The 3 new fields exist: algo, ajna_version, public_key (all String)
   — No #[serde(default)] or Serialize/Deserialize derives added
   — Existing canonical_bytes() logic is NOT broken (new fields are NOT in canonical_bytes)

7. ALL ScanResult construction sites
   — Verify these 6 files all include the 3 new fields:
     core/src/field_parser.rs, core/tests/crypto_pqc_tests.rs,
     core/tests/session_state_tests.rs, core/benches/crypto_bench.rs,
     core/benches/pipeline_bench.rs, core/src/ffi.rs
   — If any is missing the 3 new fields, output the corrected file in full

## Output format — STRICT
Output modified files as:
/// FILE: <relative/path/from/repo/root>
<complete file content — no ellipsis, no truncation>

Output passing checks as a single line: ✓ [filename] — no issues

Then output exactly this line and nothing after it:
✅ Phase 3 complete — switch model to Gemini 3.1 Pro (High) and paste Prompt 4


═══════════════════════════════════════════════════════════════
PROMPT 4 OF 4
SET ANTIGRAVITY MODEL TO: Gemini 3.1 Pro (High)
PHASE: Build, test, commit, push, CI loop to green
═══════════════════════════════════════════════════════════════

You are an execution agent with terminal access. Phases 1-3 have written all source
files for ADR-001. Your job is to reach zero failing GitHub Actions checks.
You run terminal commands. You do not write Rust or C++ code — that goes to Sonnet.

## Starting verification — run all four, output all results before proceeding

gh auth status
cargo --version
rustc --version
git status

HARD STOP: If gh auth status fails, output:
"STOP — gh CLI not authenticated. Run: gh auth login — then restart Prompt 4."
Do not proceed past a failed auth check.

## Phase 4A: workspace build

Run: cargo build --workspace 2>&1

SUCCESS → proceed to Phase 4B.

FAILURE → apply fix loop:
  1. Output the full stderr
  2. Output this block verbatim:
     ─────────────────────────────────────────────
     SWITCH TO CLAUDE SONNET 4.6 (THINKING) AND PASTE:

     Fix this Rust compile error. Output ONLY corrected file contents.
     First line of each block must be /// FILE: <path>
     followed by the complete corrected file content.
     Do not change anything not causing the error.
     No prose. No markdown fences.

     [paste full stderr here]
     ─────────────────────────────────────────────
  3. Wait. Apply fix files using the file write tool.
  4. Re-run: cargo build --workspace 2>&1
  5. Repeat up to 3 attempts on the same error.
  6. After 3 failed attempts: output "ESCALATE: build loop failed 3 times — [error]"
     and halt.

## Phase 4B: test suite

Run: cargo test --workspace 2>&1

SUCCESS → proceed to Phase 4C.
FAILURE → apply identical fix loop as Phase 4A. Same 3-attempt limit. Same ESCALATE condition.

## Phase 4C: commit and push

Run in sequence — stop and report if any command exits non-zero:

git add crates/ajna-crypto/
git add core/src/ffi.rs
git add core/src/result.rs
git add core/src/field_parser.rs
git add core/Cargo.toml
git add core/tests/crypto_pqc_tests.rs
git add core/tests/session_state_tests.rs
git add core/benches/crypto_bench.rs
git add core/benches/pipeline_bench.rs
git add backend/src/routes/session.rs
git add backend/src/routes/mod.rs
git add backend/Cargo.toml
git add Cargo.toml
git add Cargo.lock
git status

Confirm staged files match what Phases 1-3 produced. Then:

git commit -m "feat(crypto): ADR-001 cryptographic agility registry

- Add ajna-crypto crate: AjnaSigner trait, SignerRegistry, SignerError
- Implement EdDsaSigner (Ed25519, default) and MlDsaSigner (Dilithium-3, pqc feature)
- Stub EcdsaSigner and HybridSigner returning NotImplemented (Phase 2)
- Wire SignerRegistry into core/src/ffi.rs replacing hardwired PqcSigner::generate()
- C++ bridges (tflite_bridge.cpp, coreml_bridge.mm) unchanged — signing stays in Rust layer
- Add POST /v1/session/init algorithm negotiation endpoint
- Update ScanResult: add algo, ajna_version, public_key fields
- Update all ScanResult construction sites (tests, benches, field_parser)
- Register ajna-crypto in workspace Cargo.toml

Resolves: ADR-001"

git push origin main

## Phase 4D: CI watch loop

Run: gh run watch --exit-status

EXITS 0 → output:
✅ ALL CHECKS GREEN — ADR-001 implementation complete. Zero failing GitHub checks.
Halt.

EXITS NON-ZERO → apply CI fix loop:
  1. Run: gh run view --log-failed
  2. Output the full log
  3. Output this block verbatim:
     ─────────────────────────────────────────────
     SWITCH TO CLAUDE SONNET 4.6 (THINKING) AND PASTE:

     GitHub CI failed. Fix ONLY what the log shows.
     Do not refactor anything not causing the failure.
     Output each corrected file preceded by /// FILE: <path>
     Then output: FIX SUMMARY: [one sentence]

     [paste full gh run view --log-failed output here]
     ─────────────────────────────────────────────
  4. Wait. Apply fix files using the file write tool.
  5. Switch back to Gemini 3.1 Pro (High).
  6. Run:
     git add -u
     git commit -m "fix(crypto): resolve CI — [first line of error message]"
     git push origin main
  7. Run: gh run watch --exit-status
  8. Repeat loop.

After 3 consecutive pushes still failing the same check:
Output: "ESCALATE: check [name] failing after 3 fix attempts — human review required"
Halt.

## Hard constraints — never violate
- NEVER run git push --force or git push --no-verify
- NEVER delete any file not created or modified in Phases 1-3
- NEVER touch .github/workflows/, Dockerfile, or any infrastructure file
- NEVER run cargo fix --allow-dirty
- NEVER proceed past a failed gh auth status


═══════════════════════════════════════════════════════════════
QUICK REFERENCE — MODEL SWITCH CHECKLIST
═══════════════════════════════════════════════════════════════

Before pasting each prompt, confirm the model is set correctly:

[ ] Prompt 1 → Claude Opus 4.6 (Thinking)    trait + global_registry + workspace fix
[ ] Prompt 2 → Claude Sonnet 4.6 (Thinking)  signers + endpoint + ScanResult + ALL fixups
[ ] Prompt 3 → Claude Opus 4.6 (Thinking)    ffi.rs wiring + core/Cargo.toml + review
[ ] Prompt 4 → Gemini 3.1 Pro (High)         build + CI loop to green

CI fix loop (during Prompt 4 only):
  Gemini hits failure → switch to Sonnet → get fix → switch back to Gemini → continue

Each prompt ends with the exact model and prompt number to use next.
You are the router.
