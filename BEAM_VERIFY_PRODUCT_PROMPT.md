# BEAM VERIFY SDK — Full Product Build Brief
## Google Antigravity 2.0 | Multi-Session Manager View
## Version: 4.0-product | Mode: Plan-Review-Execute per session

---

## EXECUTIVE CONTEXT FOR ORCHESTRATOR

Beam is a cross-platform identity document scanning SDK built by Surt AI.
The SDK core (Rust business logic, C++ ML boundary, platform adapters, CI) is complete
and all discrepancies have been fixed. What remains is the full product package needed
to launch commercially as "Beam Verify SDK + Verification Service."

This brief is structured as FOUR SEQUENTIAL SESSIONS, each with up to 5 parallel subagents.
Sessions must run in order. Each session's output is a prerequisite for the next.
Within a session, subagents run in parallel unless a dependency is explicitly listed.

The commercial product is named: Beam Verify
Core pitch: "Mobile-native document capture with post-quantum signed verification artifacts"
Target buyers: fintech onboarding, regulated gambling KYC, KYC API vendors, telecom onboarding

---

## GLOBAL RULES FOR ALL SESSIONS AND SUBAGENTS

1. Never overwrite files that appear in the PROTECTED FILES list at the top of each session.
2. Correct wording for cryptography: use "NIST-standardised" not "FIPS certified."
   Beam implements the CRYSTALS-Dilithium algorithm as standardised in FIPS 204 (ML-DSA).
   Product certification/validation by NIST is a separate process Surt has not undertaken.
3. Correct wording for key storage: say "mlock()-protected in-process key storage with
   hardware security element (Secure Enclave / StrongBox) support on the Phase 2 roadmap."
   Do NOT say Secure Enclave or StrongBox is currently implemented — it is not.
4. Signature size: Dilithium-3 produces 3,309 bytes (pqcrypto-dilithium 0.5 / PQClean Round-3).
   FIPS 204 specifies 3,293 bytes for ML-DSA-65. Note this distinction wherever relevant.
5. Every security document must include an explicit "Out of Scope" section listing:
   - Platform hardware security element integration (Phase 2)
   - NIST CMVP / FIPS validation (separate commercial process)
   - Biometric liveness detection (separate product)
6. All monetary amounts in USD. Pricing docs target the following tiers:
   - Startup: $49/mo + $0.08/verification (up to 10k checks/mo)
   - Growth:  $249/mo + $0.05/verification (up to 100k checks/mo)
   - Enterprise: custom contract, minimum $2,000/mo, volume pricing from $0.02/verification
7. The git diff scope check runs after every session. Any file changed outside the
   session's listed targets is a failure requiring revert and investigation.

---

## SESSION ALPHA — FOUNDATIONAL BLOCKERS
## Prerequisites: none. Run first.
## Subagents: 5 parallel from start.

### ALPHA PROTECTED FILES (read only, do not modify)
```
core/src/lib.rs
core/src/frame.rs
core/src/quality.rs
core/src/session.rs
core/src/result.rs
core/src/pipeline.rs
core/src/ffi.rs
platform/android/BeamCameraAdapter.kt
platform/ios/BeamCameraAdapter.swift
build/CMakeLists.txt
```

---

### ALPHA SUBAGENT 1 — Inference output schema + field parser
Owner: Claude Opus 4.6
Dependency: none
Targets: platform/android/tflite_bridge.cpp,
          platform/ios/coreml_bridge.mm,
          platform/wasm/onnx_bridge.cpp,
          model/schema/output_schema.json (CREATE),
          core/src/field_parser.rs (CREATE)

#### Task A1.1 — Define canonical output schema

Create model/schema/output_schema.json defining the contract between any document
recognition model and the Beam inference bridges. This schema is the single source of
truth that model training, model validation, and field parsing all reference.

Structure:

```json
{
  "schema_version": "1.0.0",
  "description": "Beam Verify canonical model output schema. Any model used with Beam must produce outputs matching this specification.",
  "inputs": {
    "primary": {
      "name": "image",
      "type": "float32",
      "shape": [1, 3, "H", "W"],
      "normalisation": "divide_by_255",
      "colour_space": "RGB",
      "note": "NV12 to RGB conversion performed by bridge before tensor creation"
    }
  },
  "outputs": {
    "confidence": {
      "name": "confidence",
      "type": "float32",
      "shape": [1],
      "range": [0.0, 1.0],
      "description": "Overall scan confidence across all extracted fields"
    },
    "document_type": {
      "name": "document_type",
      "type": "int32",
      "shape": [1],
      "values": {"0": "passport", "1": "driving_licence", "2": "national_id", "3": "residence_permit", "4": "unknown"}
    },
    "issuing_country": {
      "name": "issuing_country",
      "type": "string",
      "shape": [1],
      "format": "ISO 3166-1 alpha-3"
    },
    "fields": {
      "surname": {"type": "string", "confidence_output": "surname_conf"},
      "given_names": {"type": "string", "confidence_output": "given_names_conf"},
      "date_of_birth": {"type": "string", "format": "YYYY-MM-DD", "confidence_output": "dob_conf"},
      "document_number": {"type": "string", "confidence_output": "doc_num_conf"},
      "expiry_date": {"type": "string", "format": "YYYY-MM-DD", "confidence_output": "expiry_conf"},
      "mrz_line1": {"type": "string", "confidence_output": "mrz1_conf"},
      "mrz_line2": {"type": "string", "confidence_output": "mrz2_conf"},
      "sex": {"type": "string", "values": ["M", "F", "X", ""], "confidence_output": "sex_conf"},
      "nationality": {"type": "string", "format": "ISO 3166-1 alpha-3", "confidence_output": "nat_conf"}
    },
    "fraud_signals": {
      "is_screen_photo": {"type": "float32", "range": [0.0, 1.0]},
      "is_printed_fake": {"type": "float32", "range": [0.0, 1.0]},
      "mrz_checksum_valid": {"type": "bool"}
    }
  },
  "compatibility": {
    "min_beam_version": "0.1.0",
    "tflite_opset": 17,
    "coreml_spec_version": 7,
    "onnx_opset": 17
  }
}
```

Also create model/schema/vocab_stub.json as a placeholder showing the structure any
real vocab/string-decode file must follow:

```json
{
  "version": "1.0.0",
  "note": "This stub is replaced by a real vocab file when a trained model is provided.",
  "document_type_map": {"0": "passport", "1": "driving_licence", "2": "national_id", "3": "residence_permit", "4": "unknown"},
  "string_decode": "not_applicable_for_direct_string_outputs",
  "country_code_map": "iso3166_alpha3_standard"
}
```

#### Task A1.2 — Write core/src/field_parser.rs

Create core/src/field_parser.rs implementing the bridge between raw model output
(as CField structs from the C++ layer) and validated ScanResult.

The parser must:
- Accept a Vec<CField> from ffi.rs
- Validate field values against the schema (date format, country code length, MRZ checksum)
- Perform MRZ check digit validation per ICAO 9303 Part 3 standard
- Return ParsedDocument containing all validated fields plus fraud_signals
- Return ParseError with a reason string on validation failure

MRZ check digit algorithm (ICAO 9303):
```
weights = [7, 3, 1] repeating
character_values: 0-9 = 0-9, A-Z = 10-35, < = 0
check_digit = sum(char_value * weight) % 10
```

Implement mrz_check_digit(s: &str) -> u8 and validate_mrz_line1/line2.

Add to core/src/lib.rs re-exports:
  pub use field_parser::{FieldParser, ParsedDocument, ParseError};

#### Task A1.3 — Complete decode_output_fields() in tflite_bridge.cpp

Find the stub in tflite_bridge.cpp:
  int decode_output_fields(TfLiteTensor* scores, TfLiteTensor* strings,
      CField* out_fields, int max_fields)
  { (void)scores; (void)strings; ... return 0; }

Replace the stub body with real output parsing per the schema:

```cpp
int decode_output_fields(
    TfLiteTensor* scores,
    TfLiteTensor* strings,
    CField*       out_fields,
    int           max_fields)
{
    // Output tensor layout per schema/output_schema.json:
    // Tensor 0: confidence (float32 [1])
    // Tensor 1: document_type (int32 [1])
    // Tensor 2: field_strings — N null-terminated UTF-8 strings packed sequentially
    // Tensor 3: field_confidences — float32 [N] parallel to field_strings
    //
    // This implementation expects a model following the Beam canonical schema.
    // Substitute your own parsing logic if your model uses a different output layout.

    if (!scores || !strings) return 0;

    static const char* field_keys[] = {
        "surname", "given_names", "date_of_birth", "document_number",
        "expiry_date", "mrz_line1", "mrz_line2", "sex", "nationality"
    };
    static const int N_FIELDS = 9;
    if (max_fields < N_FIELDS) return 0;

    // Parse field confidences from scores tensor
    const float* confs = scores->data.f;
    int n_confs = scores->bytes / sizeof(float);

    // Parse field strings from strings tensor
    // Strings are packed: [str1\0str2\0str3\0...]
    const char* str_data = reinterpret_cast<const char*>(strings->data.raw);
    size_t str_total = strings->bytes;

    int field_count = 0;
    size_t offset = 0;

    for (int i = 0; i < N_FIELDS && field_count < max_fields && offset < str_total; ++i) {
        const char* field_val = str_data + offset;
        size_t field_len = strnlen(field_val, str_total - offset);

        // Skip empty fields (model did not detect this field)
        if (field_len == 0) {
            offset += 1; // skip null terminator
            continue;
        }

        out_fields[field_count].key        = reinterpret_cast<const uint8_t*>(field_keys[i]);
        out_fields[field_count].key_len    = strlen(field_keys[i]);
        out_fields[field_count].value      = reinterpret_cast<const uint8_t*>(field_val);
        out_fields[field_count].value_len  = field_len;
        out_fields[field_count].confidence = (i < n_confs) ? confs[i] : 0.5f;

        offset += field_len + 1; // advance past null terminator
        ++field_count;
    }

    return field_count;
}
```

Apply the identical parsing pattern to coreml_bridge.mm (replacing the
"// Production code would iterate output.featureNames" comment block) and
onnx_bridge.cpp (replacing the confidence-only output block).

#### Verification — Alpha Subagent 1
  cargo test --release -p beam-core field_parser 2>&1 | grep -E "ok|FAILED"
Expected: field_parser tests pass. Zero FAILED.

---

### ALPHA SUBAGENT 2 — Real criterion benchmarks
Owner: Claude Opus 4.6
Dependency: none
Targets: core/benches/pipeline_bench.rs (CREATE),
          core/benches/crypto_bench.rs (CREATE),
          core/Cargo.toml (append [[bench]] entries)

#### Task A2.1 — Write pipeline_bench.rs

Create core/benches/pipeline_bench.rs using criterion 0.5.

Benchmarks required:

1. bench_blur_gate_1080p
   - Allocate 1920×1080 Y-plane (checkerboard pattern for realistic variance)
   - Construct RawFrame pointing at it
   - Benchmark QualityGate::evaluate() for 1000 iterations
   - Report mean, min, max in microseconds
   - Assert: p99 < 4000 microseconds (4ms budget for Cortex-A55 scaled from host)

2. bench_full_gate_pipeline_1080p
   - Same setup
   - Benchmark all four gates running to Accepted state
   - Assert: p99 < 8000 microseconds (host machine is faster than A55; this leaves margin)

3. bench_pipeline_rejected_at_blur
   - Use a constant-value Y-plane (guaranteed blur rejection)
   - Verify that pipeline exits in <500 microseconds (short-circuit working)

4. bench_session_state_machine
   - Create ScanSession, call record_quality_frame() × 3, complete()
   - Benchmark: < 100 microseconds per full session lifecycle (excluding inference)

Each benchmark must include a criterion throughput report (Throughput::Elements(1)).

```rust
use criterion::{criterion_group, criterion_main, Criterion, Throughput, BenchmarkId};
use beam_core::{QualityGate, RawFrame, PixelFormat};
use std::time::Duration;

fn make_checkerboard_frame(w: u32, h: u32) -> (Vec<u8>, Vec<u8>) {
    let y: Vec<u8> = (0..(w*h) as usize)
        .map(|i| if (i/w as usize + i%w as usize) % 2 == 0 { 220 } else { 30 })
        .collect();
    let uv = vec![128u8; (w*h/2) as usize];
    (y, uv)
}

pub fn bench_blur_gate_1080p(c: &mut Criterion) {
    let (y, uv) = make_checkerboard_frame(1920, 1080);
    let frame = RawFrame {
        y_plane: y.as_ptr(), uv_plane: uv.as_ptr(),
        width: 1920, height: 1080, y_stride: 1920, uv_stride: 1920,
        format: PixelFormat::Nv12, timestamp_us: 0,
    };
    let mut gate = QualityGate::default();
    let mut group = c.benchmark_group("quality_gates");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(5));
    group.bench_function("blur_gate_1080p", |b| {
        b.iter(|| unsafe { gate.evaluate(&frame) })
    });
    group.finish();
}
```

Add to core/Cargo.toml after [dev-dependencies]:

```toml
[[bench]]
name = "pipeline_bench"
harness = false

[[bench]]
name = "crypto_bench"
harness = false
```

#### Task A2.2 — Write crypto_bench.rs

Create core/benches/crypto_bench.rs benchmarking:

1. bench_ml_dsa_keygen — PqcSigner::generate(MlDsaLevel::Level3)
   - Target: < 5ms per keygen on host (much slower on A55, but documents the cost)

2. bench_ml_dsa_sign — sign 256-byte message
   - Target: < 5ms per sign on host

3. bench_ml_dsa_verify — verify a 3,309-byte signature
   - Target: < 3ms per verify on host

4. bench_canonical_bytes — ScanResult::canonical_bytes() with 9 fields
   - Target: < 100 microseconds

5. bench_ml_kem_encapsulate — MlKemSession::encapsulate()
   - Target: < 3ms per encapsulation

#### Verification — Alpha Subagent 2
  cd core && cargo bench --bench pipeline_bench 2>&1 | grep -E "time:|FAILED|error"
Expected: bench output shows time measurements. No errors.

---

### ALPHA SUBAGENT 3 — Security documentation correction
Owner: Claude Sonnet 4.6
Dependency: none
Targets: docs/SECURITY_MODEL.md,
          docs/BEAM_WHITEPAPER.md,
          docs/WHITEPAPER.md

Read all three files completely before making any edit.
Change only the specific items listed below. Do not restructure, rewrite, or expand.

#### Task A3.1 — Fix "Secure Enclave / StrongBox" wording

In docs/SECURITY_MODEL.md, find the "Key Storage Per Platform" table.
The iOS row currently says: "Private key never leaves the enclave."
Replace the entire iOS row with:
  Platform: iOS
  Key Storage: mlock()-protected in-process (Phase 1). Secure Enclave integration on Phase 2 roadmap.
  Notes: Private key is generated by pqcrypto-dilithium and held in process memory.
         mlock() prevents swap to disk. Hardware enclave integration planned for Phase 2.

The Android row currently says: "Hardware-backed key storage via Android Keystore."
Replace with:
  Platform: Android
  Key Storage: mlock()-protected in-process (Phase 1). StrongBox Keymaster integration on Phase 2 roadmap.
  Notes: Same as iOS — pqcrypto-dilithium keygen in process, mlock-protected, hardware
         key store planned for Phase 2 via Android Keystore API.

The WASM row: leave unchanged — it is already honest about in-memory limitation.

Find any sentence in docs/SECURITY_MODEL.md containing "Secure Enclave" or "StrongBox"
that implies current implementation. Replace with a sentence making clear these are
Phase 2 roadmap items.

#### Task A3.2 — Fix "FIPS certified" and related wording

Search all three target files for:
  - "FIPS certified"
  - "FIPS validated"
  - "FIPS compliant"
  - "certified implementation"

Replace every occurrence with one of:
  - "NIST-standardised" (when describing the algorithm)
  - "based on NIST FIPS 204" (when describing implementation)
  - "implementing the NIST standard" (when describing the crate)

Also find any sentence that says the pqcrypto-dilithium crate IS the FIPS 204 standard.
Replace with: "pqcrypto-dilithium 0.5 implements CRYSTALS-Dilithium (ML-DSA) as specified
in NIST FIPS 204, using the PQClean Round-3 reference implementation. This crate has not
undergone NIST CMVP validation; product-level FIPS validation is on the Phase 3 roadmap."

#### Task A3.3 — Add "Out of Scope" section to SECURITY_MODEL.md

At the end of docs/SECURITY_MODEL.md, add a new section:

## Out of scope for Beam Verify v1.0

The following security capabilities are explicitly excluded from the current release
and are documented here to prevent misrepresentation in sales or compliance contexts:

1. Platform hardware security element integration — Secure Enclave (iOS) and
   StrongBox Keymaster (Android) integration is planned for Phase 2. Current release
   uses mlock()-protected in-process key storage.

2. NIST CMVP / FIPS 140 validation — The pqcrypto-dilithium crate implements a
   NIST-standardised algorithm but has not undergone NIST Cryptographic Module
   Validation Program testing. CMVP validation is on the Phase 3 roadmap.

3. Biometric liveness detection — Face match and liveness are provided by FaceGuard,
   a separate Surt AI product not included in Beam Verify.

4. NFC chip reading — ICAO 9303 NFC-based chip authentication is on the Phase 2 roadmap.

5. Government database lookups — Document authenticity is assessed on-device only.
   External database cross-checks are a separate backend service not included in v1.0.

#### Verification — Alpha Subagent 3
  grep -in "fips certified\|fips validated\|fips compliant\|never leaves the enclave\|strongbox keymaster" \
    docs/SECURITY_MODEL.md docs/BEAM_WHITEPAPER.md docs/WHITEPAPER.md | wc -l
Expected: 0

---

### ALPHA SUBAGENT 4 — Backend service foundation
Owner: Claude Opus 4.6
Dependency: none
Targets: backend/ directory (CREATE entirely)

The verification backend is a new service. It does NOT modify any existing SDK files.
Language: Rust (Axum web framework). Target: deployable to fly.io, Railway, or any
container platform. PostgreSQL for persistence. Redis for nonce cache.

Create this directory structure and all listed files:

```
backend/
  Cargo.toml
  src/
    main.rs
    routes/
      mod.rs
      nonce.rs
      verify.rs
      audit.rs
      webhook.rs
      health.rs
    models/
      mod.rs
      nonce.rs
      verification_result.rs
      audit_log.rs
      webhook_config.rs
    crypto/
      mod.rs
      ml_dsa_verifier.rs
    db/
      mod.rs
      migrations/
        001_initial.sql
    config.rs
    errors.rs
  Dockerfile
  docker-compose.yml (for local dev)
```

#### Task A4.1 — backend/Cargo.toml

```toml
[package]
name = "beam-verify-backend"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio", "uuid", "time"] }
redis = { version = "0.24", features = ["tokio-comp"] }
uuid = { version = "1", features = ["v4", "serde"] }
time = { version = "0.3", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
pqcrypto-dilithium = "0.5"
pqcrypto-traits = "0.3"
anyhow = "1"
thiserror = "1"
reqwest = { version = "0.11", features = ["json"] }
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
dotenvy = "0.15"
```

#### Task A4.2 — API routes

Implement these five routes:

POST /v1/nonce
  Input: { "session_id": "uuid4" }
  Action: Generate a 32-byte random nonce, store in Redis with TTL 300s keyed by session_id.
  Output: { "nonce": "hex64", "expires_at": "iso8601", "session_id": "uuid4" }

POST /v1/verify
  Input: multipart or JSON:
    session_id: uuid
    nonce: hex string (must match Redis record for this session)
    scan_result: {
      fields: [...],
      document_type: string,
      issuing_country: string,
      confidence: float,
      pqc_signature: base64,
      pqc_public_key: base64
    }
  Action:
    1. Retrieve nonce from Redis by session_id. If not found: 410 Gone.
    2. Delete nonce from Redis immediately (single-use enforcement).
    3. Reconstruct canonical_bytes from scan_result fields (must match client implementation).
    4. Verify ML-DSA signature using pqcrypto-dilithium::dilithium3::verify_detached_signature.
    5. If valid: persist to verification_results table, return 200 with verified:true.
    6. If invalid: return 401 with verified:false and reason.
    7. Write audit log regardless of outcome.
  Output: {
    "verified": bool,
    "session_id": "uuid",
    "verification_id": "uuid",
    "document_type": string,
    "issuing_country": string,
    "confidence": float,
    "fraud_signals": {...},
    "timestamp": "iso8601"
  }

GET /v1/audit
  Query params: session_id (optional), from (iso8601), to (iso8601), limit (default 100)
  Returns: paginated list of audit log entries for the authenticated API key's tenant.

POST /v1/webhooks
  Input: { "url": "https://...", "events": ["verification.complete", "verification.failed"],
           "secret": "optional_hmac_secret" }
  Action: Register webhook URL for this tenant. Validate URL reachability.
  Output: { "webhook_id": "uuid", "url": string, "events": [...], "created_at": "iso8601" }

GET /health
  Returns: { "status": "ok", "db": "ok|degraded", "redis": "ok|degraded", "version": "0.1.0" }

#### Task A4.3 — Webhook delivery

When a verification completes, dispatch a webhook to all registered URLs for the tenant:

Payload:
```json
{
  "event": "verification.complete",
  "verification_id": "uuid",
  "session_id": "uuid",
  "timestamp": "iso8601",
  "document_type": "passport",
  "issuing_country": "USA",
  "confidence": 0.94,
  "verified": true
}
```

Sign the payload with HMAC-SHA256 using the tenant's webhook secret.
Include the signature as X-Beam-Signature: sha256=hexdigest header.
Retry on failure: 3 attempts with exponential backoff (1s, 5s, 25s).
Dead letter failures to the audit log with reason.

#### Task A4.4 — Database migrations

Create backend/src/db/migrations/001_initial.sql:

```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE tenants (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        TEXT NOT NULL,
    api_key     TEXT NOT NULL UNIQUE,
    plan        TEXT NOT NULL DEFAULT 'startup',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE verification_results (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id           UUID NOT NULL REFERENCES tenants(id),
    session_id          UUID NOT NULL,
    document_type       TEXT,
    issuing_country     TEXT,
    confidence          FLOAT,
    pqc_verified        BOOLEAN NOT NULL DEFAULT FALSE,
    pqc_public_key_hex  TEXT,
    fraud_signals       JSONB,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE audit_logs (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id),
    session_id      UUID,
    event_type      TEXT NOT NULL,
    outcome         TEXT NOT NULL,
    detail          JSONB,
    ip_address      INET,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE webhook_configs (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id   UUID NOT NULL REFERENCES tenants(id),
    url         TEXT NOT NULL,
    events      TEXT[] NOT NULL,
    secret_hex  TEXT,
    active      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_tenant_time ON audit_logs(tenant_id, created_at DESC);
CREATE INDEX idx_verif_tenant_session ON verification_results(tenant_id, session_id);
```

#### Task A4.5 — Dockerfile and docker-compose

Dockerfile (multi-stage, minimal final image):
```dockerfile
FROM rust:1.78-slim AS builder
WORKDIR /app
COPY Cargo.toml .
COPY src/ src/
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/beam-verify-backend /usr/local/bin/
EXPOSE 8080
CMD ["beam-verify-backend"]
```

docker-compose.yml for local dev:
```yaml
version: "3.9"
services:
  api:
    build: .
    ports: ["8080:8080"]
    environment:
      DATABASE_URL: postgres://beam:beam@db:5432/beam_verify
      REDIS_URL: redis://redis:6379
    depends_on: [db, redis]
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: beam_verify
      POSTGRES_USER: beam
      POSTGRES_PASSWORD: beam
    ports: ["5432:5432"]
  redis:
    image: redis:7-alpine
    ports: ["6379:6379"]
```

#### Verification — Alpha Subagent 4
  cd backend && cargo check 2>&1 | grep -E "^error" | wc -l
Expected: 0

---

### ALPHA SUBAGENT 5 — Model pack infrastructure
Owner: Claude Sonnet 4.6
Dependency: none
Targets: model/ directory (CREATE)

```
model/
  README.md
  schema/
    output_schema.json     (written by Subagent 1)
    vocab_stub.json        (written by Subagent 1)
  manifest/
    model_manifest.json    (CREATE here)
  signing/
    sign_model.sh          (CREATE here)
    verify_model.sh        (CREATE here)
    README.md              (CREATE here)
  placeholder/
    README.md              (CREATE here)
    PLACEHOLDER.tflite     (empty 4-byte file: CREATE here)
    PLACEHOLDER.mlpackage/ (empty dir: CREATE here)
    PLACEHOLDER.onnx       (empty 4-byte file: CREATE here)
```

#### Task A5.1 — model/manifest/model_manifest.json

```json
{
  "manifest_version": "1.0.0",
  "description": "Beam Verify model pack manifest. This file describes the model artifacts that must be present for a complete Beam Verify deployment.",
  "beam_sdk_version": "0.1.0",
  "models": [
    {
      "platform": "android",
      "filename": "beam_idv_v1.tflite",
      "sha256": "REPLACE_WHEN_MODEL_IS_PROVIDED",
      "size_bytes": 0,
      "input_name": "image",
      "output_names": ["confidence", "document_type", "issuing_country", "field_strings", "field_confidences", "fraud_signals"],
      "opset": 17,
      "quantised": true,
      "quantisation": "int8",
      "target_devices": ["arm64-v8a", "armeabi-v7a"],
      "tested_devices": ["PLACEHOLDER_fill_before_release"]
    },
    {
      "platform": "ios",
      "filename": "beam_idv_v1.mlpackage",
      "sha256": "REPLACE_WHEN_MODEL_IS_PROVIDED",
      "size_bytes": 0,
      "input_name": "image",
      "compute_units": "all",
      "coreml_spec_version": 7,
      "supports_ane": true
    },
    {
      "platform": "wasm",
      "filename": "beam_idv_v1.onnx",
      "sha256": "REPLACE_WHEN_MODEL_IS_PROVIDED",
      "size_bytes": 0,
      "input_name": "image",
      "opset": 17,
      "webgpu_compatible": true
    }
  ],
  "vocab": {
    "filename": "vocab.json",
    "sha256": "REPLACE_WHEN_MODEL_IS_PROVIDED"
  },
  "schema": {
    "filename": "output_schema.json",
    "sha256": "COMPUTE_WITH_sha256sum_output_schema.json"
  },
  "release_date": "REPLACE_BEFORE_RELEASE",
  "minimum_confidence_threshold": 0.85,
  "supported_document_types": ["passport", "driving_licence", "national_id"],
  "supported_countries": "See docs/supported_countries.md (to be created with model)"
}
```

#### Task A5.2 — model/signing/sign_model.sh

```bash
#!/usr/bin/env bash
# sign_model.sh — Sign a Beam model artifact with an Ed25519 key.
# Usage: ./sign_model.sh beam_idv_v1.tflite signing_key.pem
# Produces: beam_idv_v1.tflite.sig (detached Ed25519 signature)
# NOTE: Model signing uses classical Ed25519 for the signing infrastructure itself.
# The Beam result-level signature uses ML-DSA (NIST FIPS 204). These serve
# different purposes: Ed25519 authenticates that Surt produced the model file;
# ML-DSA proves that a specific scan result was produced by a genuine Beam device.
set -euo pipefail
MODEL="$1"
KEY="$2"
if [ ! -f "${MODEL}" ] || [ ! -f "${KEY}" ]; then
    echo "Usage: $0 <model_file> <signing_key.pem>"
    exit 1
fi
openssl dgst -sha256 -sign "${KEY}" -out "${MODEL}.sig" "${MODEL}"
SHA256=$(sha256sum "${MODEL}" | awk '{print $1}')
echo "Signed: ${MODEL}"
echo "SHA256: ${SHA256}"
echo "Signature written to: ${MODEL}.sig"
echo "Update manifest/model_manifest.json sha256 field with: ${SHA256}"
```

#### Task A5.3 — model/placeholder/README.md

Write a clear README explaining that the three PLACEHOLDER files are intentional
empty artifacts showing the required filename pattern. Integrators must replace them
with real model files following the schema in schema/output_schema.json.
Include: required input/output tensor names, minimum size expectations, and a link
to model/manifest/model_manifest.json for the full contract.

#### Verification — Alpha Subagent 5
  python3 -c "import json; json.load(open('model/manifest/model_manifest.json')); print('manifest valid')"
Expected: "manifest valid"
  jq . model/schema/output_schema.json > /dev/null && echo "schema valid"
Expected: "schema valid"

---

## SESSION BETA — SDK PUBLISHING + SAMPLE APPS
## Prerequisites: Session Alpha complete and all 5 Alpha verifications pass.
## Subagents: 4 parallel.

### BETA PROTECTED FILES (read only)
All files from Session Alpha protected list plus all new files from Alpha.

---

### BETA SUBAGENT 1 — SDK publishing pipeline (Maven + SPM + npm)
Owner: Claude Sonnet 4.6
Targets: publishing/ directory (CREATE),
          .github/workflows/publish.yml (CREATE)

Create three publishing scripts and one unified GitHub Actions publish workflow.

#### Task B1.1 — publishing/publish_android_maven.sh

Publishes the Android AAR to GitHub Packages (Maven) or Maven Central.
Must:
- Read version from Cargo.toml (cargo metadata --format-version 1)
- Generate POM file with groupId ai.surt, artifactId beam-verify-android, version from Cargo
- Sign with GPG if MAVEN_GPG_KEY is set in environment
- Upload to MAVEN_REPO_URL (default: GitHub Packages)
- Print: "Published BeamVerify Android v{version} to {repo}"

#### Task B1.2 — publishing/publish_ios_spm.sh

Creates a Swift Package Manager manifest (Package.swift) pointing to the XCFramework.
Must:
- Read version from Cargo.toml
- Generate Package.swift with:
    name: BeamVerify
    platforms: [.iOS(.v15)]
    products: [.library(name: "BeamVerify", targets: ["BeamVerify"])]
    targets: [.binaryTarget(name: "BeamVerify",
                url: "{RELEASE_URL}/BeamSDK-{version}.xcframework.zip",
                checksum: "{sha256}")]
- Create the XCFramework zip and compute its sha256
- Tag the git repo with v{version} so SPM can resolve it
- Print: "Package.swift generated. Tag v{version} and push to enable SPM resolution."

#### Task B1.3 — publishing/publish_npm.sh

Extends scripts/package_wasm_npm.sh to do a real npm publish.
Must:
- Call the existing package_wasm_npm.sh
- Set version in dist/npm/package.json from Cargo.toml
- Run npm publish --access public (or --dry-run if NPM_DRY_RUN=1)
- Print: "Published @surt/beam-sdk v{version} to npm"

#### Task B1.4 — .github/workflows/publish.yml

```yaml
name: Publish Beam Verify SDK
on:
  push:
    tags: ['v*']
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to publish (e.g. 0.1.0)'
        required: true

jobs:
  publish-android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build AAR
        run: bash scripts/package_android_aar.sh
      - name: Publish to Maven
        run: bash publishing/publish_android_maven.sh
        env:
          MAVEN_REPO_URL: ${{ secrets.MAVEN_REPO_URL }}
          MAVEN_GPG_KEY: ${{ secrets.MAVEN_GPG_KEY }}

  publish-ios:
    runs-on: macos-15
    needs: [publish-android]
    steps:
      - uses: actions/checkout@v4
      - name: Build XCFramework
        run: bash scripts/package_ios_xcframework.sh
      - name: Publish SPM
        run: bash publishing/publish_ios_spm.sh
        env:
          RELEASE_URL: ${{ secrets.RELEASE_BASE_URL }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  publish-npm:
    runs-on: ubuntu-latest
    needs: [publish-android]
    steps:
      - uses: actions/checkout@v4
      - uses: mymindstorm/setup-emsdk@v14
        with: { version: latest }
      - name: Publish npm
        run: bash publishing/publish_npm.sh
        env:
          NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

---

### BETA SUBAGENT 2 — Android sample app
Owner: Claude Opus 4.6
Targets: samples/android/ directory (CREATE)

Create a minimal but complete Android sample app demonstrating Beam Verify end-to-end.

```
samples/android/
  build.gradle (app level)
  settings.gradle
  gradle.properties
  app/
    build.gradle
    src/main/
      AndroidManifest.xml
      java/ai/surt/beam/sample/
        MainActivity.kt
        ScanActivity.kt
        ResultActivity.kt
      res/
        layout/
          activity_main.xml
          activity_scan.xml
          activity_result.xml
        values/
          strings.xml
          colors.xml
```

The sample app must:
- Show a home screen with a "Start Verification" button
- Open ScanActivity which starts BeamCameraAdapter
- Show a live preview with a document overlay guide (simple rounded rect)
- Display quality gate status (blurry / too dark / scanning / complete)
- When session completes, show ResultActivity with extracted fields table
- ResultActivity shows: document type, issuing country, confidence as a percentage,
  each field and its confidence, and the verification ID from the backend
- Include CAMERA permission request with rationale dialog
- Include a settings screen with: backend URL, API key, pqc_sign_result toggle
- Error handling: show user-friendly messages for camera permission denied,
  session timeout, and backend unreachable

All strings in strings.xml. No hardcoded text in code or layouts.

---

### BETA SUBAGENT 3 — iOS sample app
Owner: Claude Opus 4.6
Targets: samples/ios/ directory (CREATE)

Create a minimal but complete iOS sample app demonstrating Beam Verify end-to-end.

```
samples/ios/
  BeamVerifySample.xcodeproj/ (or Package.swift if SPM-based)
  BeamVerifySample/
    App.swift
    ContentView.swift
    ScanView.swift
    ResultView.swift
    BackendService.swift
    SettingsView.swift
    Info.plist
```

Using SwiftUI throughout. The sample app must:
- ContentView: "Start Verification" button, brief product description
- ScanView: AVCaptureSession preview via UIViewRepresentable, document outline overlay,
  real-time quality status label ("Hold still", "Too dark", "Scanning...", "Complete")
- ResultView: Extracted fields list, confidence gauge (SwiftUI ProgressView),
  verification badge (green check or red X based on backend response)
- BackendService: URLSession calls to POST /v1/nonce and POST /v1/verify with
  configurable base URL and API key
- SettingsView: baseURL, apiKey, pqcSignResult toggle
- NSCameraUsageDescription in Info.plist with a clear user-facing reason string

---

### BETA SUBAGENT 4 — Web/WASM sample app
Owner: Claude Sonnet 4.6
Targets: samples/web/ directory (CREATE)

```
samples/web/
  index.html
  src/
    main.ts
    scanner.ts
    backend.ts
    ui.ts
  package.json
  tsconfig.json
  vite.config.ts
```

TypeScript + Vite. No heavy framework dependency (vanilla TS, minimal dependencies).

The sample app must:
- Request camera permission and show getUserMedia preview
- On each frame: copy ImageData to WASM heap, call beam_wasm_process_frame
- Show quality gate status in a status bar below the preview
- On completion: display extracted fields, call the backend verify endpoint
- Handle the mandatory WASM copy gracefully: show a "Processing..." indicator
  that does not block the UI thread (use a Web Worker for the WASM processing)
- Mobile-responsive layout (single column on narrow viewports)

---

## SESSION GAMMA — COMPLIANCE PACK + COMMERCIAL DOCUMENTATION
## Prerequisites: Sessions Alpha and Beta complete.
## Subagents: 4 parallel.

---

### GAMMA SUBAGENT 1 — SBOM and threat model
Owner: Claude Sonnet 4.6
Targets: compliance/ directory (CREATE)

```
compliance/
  SBOM.json              (CycloneDX 1.4 format)
  THREAT_MODEL.md
  EXPORT_CONTROL.md
  FFI_FUZZING_GUIDE.md
  DEVICE_BENCHMARK_GUIDE.md
```

#### Task G1.1 — SBOM.json (CycloneDX 1.4)

Generate a CycloneDX 1.4 format SBOM listing all direct dependencies from:
- core/Cargo.toml
- backend/Cargo.toml
- platform/android/build.gradle (if present)

For each dependency include: name, version, purl, license, supplier.
Identify the pqcrypto-dilithium and pqcrypto-kyber components with:
  supplier: "PQClean project / pqcrypto Rust crate authors"
  description: "CRYSTALS-Dilithium post-quantum signature implementation"
  notes: "Based on PQClean Round-3 reference implementation. Not NIST CMVP validated."

#### Task G1.2 — THREAT_MODEL.md

Write a structured threat model covering:

1. In-scope assets: scan results, private signing keys, nonce store, audit logs
2. Threat actors: fraudsters presenting fake documents, device-level attackers,
   network-level adversaries, rogue integrators, insider threats at Surt
3. Attack surface inventory (complete — each with likelihood and impact):
   - Physical: fake documents, screen photos, printed fakes
   - Memory: buffer overread in C++ bridge, heap exhaustion on budget device
   - Cryptographic: quantum harvest-now-decrypt-later, signature replay
   - Transport: TLS downgrade, certificate substitution
   - Backend: nonce replay window, SQL injection in audit queries, webhook SSRF
   - Supply chain: malicious model weights, compromised pqcrypto crate
4. Controls currently in place for each threat
5. Controls deferred to Phase 2 or 3
6. Residual risk statement

#### Task G1.3 — EXPORT_CONTROL.md

Write a clear export control notice:

Beam Verify SDK incorporates post-quantum cryptographic algorithms (ML-DSA, ML-KEM)
standardised by NIST in FIPS 203 and FIPS 204 (August 2024). Under the US Export
Administration Regulations (EAR), these algorithms fall under ECCN 5E002.

Standard actions required before distribution:
- US persons may generally distribute to most countries under EAR99 encryption
  licensing exceptions, but must conduct a CCATS review for export to embargoed countries.
- Do not distribute to Cuba, Iran, North Korea, Syria, or Russia without specific OFAC license.
- Consult export counsel before distributing to entities on the Entity List (15 CFR Part 744).

This document does not constitute legal advice. Consult a qualified export counsel
before distributing Beam Verify SDK in any commercial context.

#### Task G1.4 — FFI_FUZZING_GUIDE.md

Write a practical guide covering:
- How to compile libbeam_core.a as a libfuzzer target
- Setting up a corpus of synthetic RawFrame data
- The key invariants to fuzz (null pointers, oversized dimensions, stride mismatches)
- How to run with AddressSanitiser + libfuzzer: cargo fuzz run beam_ffi_frame
- Expected output and how to triage crashes

Include a starter fuzz target:

```rust
// fuzz/fuzz_targets/beam_ffi_frame.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use beam_core::ffi::*;
use beam_core::frame::{RawFrame, PixelFormat};

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 { return; }
    let width  = u32::from_le_bytes(data[0..4].try_into().unwrap()).min(3840);
    let height = u32::from_le_bytes(data[4..8].try_into().unwrap()).min(2160);
    if width == 0 || height == 0 { return; }
    if (data.len() as u64) < (width as u64 * height as u64 + 16) { return; }
    let y_start = 16usize;
    let frame = RawFrame {
        y_plane: data[y_start..].as_ptr(),
        uv_plane: data[y_start..].as_ptr(),
        width, height,
        y_stride: width, uv_stride: width,
        format: PixelFormat::Nv12,
        timestamp_us: 0,
    };
    let gate = beam_gate_create();
    unsafe { beam_gate_evaluate(gate, &frame); }
    unsafe { beam_gate_destroy(gate); }
});
```

---

### GAMMA SUBAGENT 2 — Admin dashboard
Owner: Claude Sonnet 4.6
Targets: dashboard/ directory (CREATE)

A minimal web-based admin dashboard for Beam Verify customers.
Technology: Vanilla TypeScript + Vite + Chart.js. No heavy framework.
Connects to the backend API built in Session Alpha.

```
dashboard/
  index.html
  src/
    main.ts
    views/
      overview.ts
      verifications.ts
      audit.ts
      webhooks.ts
      settings.ts
    components/
      nav.ts
      stat_card.ts
      table.ts
      chart.ts
    api.ts
  package.json
  vite.config.ts
```

The dashboard must show:
- Overview: verifications today, this week, success rate, avg confidence gauge
- Verifications table: date, session_id, document type, country, confidence, verified
- Audit log: filterable by date range, event type
- Webhook configuration: list registered webhooks, add/remove
- Settings: API key display (masked), plan info, contact support link

Use the CSS variable design system from the Beam SDK guide (no framework CSS).
Light/dark mode via prefers-color-scheme.

---

### GAMMA SUBAGENT 3 — Commercial documentation
Owner: Claude Sonnet 4.6
Targets: commercial/ directory (CREATE)

```
commercial/
  PRICING.md
  BATTLECARD.md
  INTEGRATION_CHECKLIST.md
  SUPPORTED_DOCUMENTS.md
```

#### Task G3.1 — PRICING.md

Detail the three tiers:

Startup — $49/month + $0.08/verification
  Includes: Android AAR + iOS XCFramework + WASM package
  Up to: 10,000 verifications/month
  Verification backend: Surt-hosted, 99.5% SLA
  Support: community forum + email (48h response)
  Overage: $0.10/verification above 10k

Growth — $249/month + $0.05/verification
  All Startup features plus:
  Up to: 100,000 verifications/month
  Webhook delivery, audit logs, dashboard access
  Custom document types: up to 3 additional on request
  Support: email (24h response) + Slack channel
  Overage: $0.06/verification above 100k

Enterprise — custom contract, minimum $2,000/month
  All Growth features plus:
  Unlimited verifications (volume pricing from $0.02/check at 1M+/month)
  Self-hosted backend option (requires enterprise tier)
  SLA: 99.9% uptime with credits
  Priority model updates, dedicated support engineer
  SBOM, threat model, compliance pack access
  Signed BAA for HIPAA-adjacent use cases

SDK-only license (no backend)
  Customers operating their own verification backend may license the SDK
  without using Surt's verification service.
  Pricing: $500/month flat fee per deployment environment.
  No per-verification fees. Customer implements their own nonce/verify/audit.

#### Task G3.2 — BATTLECARD.md

Write a one-page sales battlecard with sections:
  "When a prospect mentions Scandit" — key differentiators, pricing comparison
  "When a prospect mentions BlinkID" — on-device parity, PQC advantage
  "When a prospect mentions Veriff" — on-device vs cloud, latency, data residency
  "When a prospect asks about FIPS compliance" — honest answer, positioning
  "When a prospect asks about liveness" — FaceGuard upsell, clear scope
  "Questions we ask first" — what is your monthly scan volume? which platform?
    any GDPR/data residency constraints? regulated industry?

#### Task G3.3 — INTEGRATION_CHECKLIST.md

A step-by-step integration checklist for developer buyers:
  Phase 1 (Day 1): Add AAR/XCFramework/npm. Run sample app. Confirm stub mode works.
  Phase 2 (Week 1): Drop model file in. Run against a real document. Verify 85%+ confidence.
  Phase 3 (Week 2): Wire backend nonce + verify. Enable PQC signing. Test replay prevention.
  Phase 4 (Launch): Enable webhooks. Set up audit log alerts. Configure dashboard.

---

### GAMMA SUBAGENT 4 — CI integration for all new modules
Owner: Claude Sonnet 4.6
Targets: .github/workflows/ci.yml (EXTEND, do not replace),
          .github/workflows/backend_ci.yml (CREATE)

Extend the existing ci.yml to add:
- A backend-check job: cd backend && cargo check --release
- A dashboard-build job: cd dashboard && npm ci && npm run build

Create .github/workflows/backend_ci.yml:
```yaml
name: Backend CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16-alpine
        env:
          POSTGRES_DB: beam_verify_test
          POSTGRES_USER: beam
          POSTGRES_PASSWORD: beam
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      redis:
        image: redis:7-alpine
        options: --health-cmd "redis-cli ping" --health-interval 10s
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Run backend tests
        run: cargo test --release
        working-directory: backend
        env:
          DATABASE_URL: postgres://beam:beam@localhost:5432/beam_verify_test
          REDIS_URL: redis://localhost:6379
```

---

## FINAL ORCHESTRATOR GATE — ALL SESSIONS COMPLETE

Run after all three sessions pass their individual verifications.

```bash
echo "=== BEAM VERIFY PRODUCT COMPLETENESS CHECK ==="

echo "--- Rust core tests ---"
cd core && cargo test --release 2>&1 | grep "test result"
cd ..

echo "--- Backend compiles ---"
cd backend && cargo check 2>&1 | grep -c "^error" | xargs -I{} test {} -eq 0 && echo "PASS" || echo "FAIL"
cd ..

echo "--- Benchmarks run ---"
cd core && cargo bench --bench pipeline_bench 2>&1 | grep -c "time:" | xargs -I{} test {} -gt 0 && echo "PASS bench" || echo "FAIL bench"
cd ..

echo "--- Security docs corrected ---"
grep -in "fips certified\|never leaves the enclave" docs/SECURITY_MODEL.md | wc -l | xargs -I{} test {} -eq 0 && echo "PASS security docs" || echo "FAIL security docs"

echo "--- Model manifest valid ---"
python3 -c "import json; json.load(open('model/manifest/model_manifest.json'))" && echo "PASS manifest"

echo "--- Sample apps exist ---"
test -f samples/android/app/src/main/java/ai/surt/beam/sample/MainActivity.kt && echo "PASS android sample"
test -f samples/ios/BeamVerifySample/ScanView.swift && echo "PASS ios sample"
test -f samples/web/src/scanner.ts && echo "PASS web sample"

echo "--- Dashboard exists ---"
test -f dashboard/src/views/overview.ts && echo "PASS dashboard"

echo "--- Commercial docs exist ---"
test -f commercial/PRICING.md && test -f commercial/BATTLECARD.md && echo "PASS commercial"

echo "--- Compliance pack exists ---"
test -f compliance/SBOM.json && test -f compliance/THREAT_MODEL.md && echo "PASS compliance"

echo "=== CHECK COMPLETE ==="
```

All lines must print PASS. Any FAIL requires investigation before the product is marked ready.

---

## WHAT REMAINS FOR THE HUMAN AFTER THIS PROMPT RUNS

1. Provide a real document recognition model (TFLite / mlpackage / ONNX).
   Update model/manifest/model_manifest.json with real sha256 values.

2. Update decode_output_fields() if your model uses a different output tensor layout
   than the canonical schema specifies.

3. Run the backend on a real server:
   - Provision PostgreSQL and Redis
   - Set DATABASE_URL and REDIS_URL environment variables
   - Run: cargo run --release from the backend/ directory

4. Generate a real Ed25519 signing key for model signing:
   openssl genpkey -algorithm ed25519 -out model_signing_key.pem
   Keep this key secret. Publish only the public key.

5. Register for npm, Maven, and Swift Package Index accounts to enable publishing.

6. Engage export counsel before any commercial distribution.

7. Plan Phase 2 engineering: Secure Enclave (iOS), StrongBox (Android), NFC chip reading.

8. Begin Surt entity graph integration: pipe POST /v1/verify results into the
   IDV, Guardian, and FaceGuard entity nodes as documented in docs/BEAM_WHITEPAPER.md.
