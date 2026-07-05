# Ajna GTM Platform — Implementation Spec & Task List

Goal: evolve the Beam edge-PQC document scanner into **Ajna**, a three-pillar GTM
security platform (IDV, Intel, Vision) per the /goal termination conditions.

## Architectural Assumptions

1. **Rebrand scope**: Cargo package names, FFI symbols (`beam_*` → `ajna_*`),
   C header, CMake, platform bridges (Swift/Kotlin/C++/TS), samples
   (`ai.surt.beam` → `com.ajna.sample`), CI workflows, docs, SBOM.
   Archives stay untouched: `beam-context/` (imported by CLAUDE.md),
   `BEAM_ANTIGRAVITY_PROMPT_v3.md`, `SURT_README.md`, `repomix-output.xml`.
   Directory names `core/` and `backend/` stay (neutral); `crates/beam-crypto`
   → `crates/ajna-crypto` via git mv.
2. **Crate layering** (no duplicated engine logic):
   - `ajna-core` (was beam-core): scanning engine, quality gates, session FSM,
     FFI, **new: declarative `UiConfig` + headless mode types**.
   - `ajna-crypto`: unchanged signer registry (Ed25519, ML-DSA-65, hybrid).
   - `ajna-idv`: product facade over ajna-core (IdvSession, UiConfig re-export,
     headless runner). Depends on ajna-core + ajna-crypto.
   - `ajna-intel`: device posture — pure functions over platform-supplied
     `DeviceIndicators` (jailbreak/root paths, emulator props, debugger,
     hooking frameworks) → risk-scored, PQC-signed `PostureReport`.
   - `ajna-vision`: liveness challenge FSM (blink/turn/smile) + face embedding
     cosine match → PQC-signed `VisionResult`. Model-agnostic (f32 embeddings).
   - `ajna-mcp-server`: binary crate, stdio JSON-RPC 2.0 MCP server exposing
     `ajna_evaluate_device_posture`, `ajna_verify_face`, `ajna_verify_document`
     (via backend REST when AJNA_BACKEND_URL set), `ajna_query_audit_log`.
     Hand-rolled protocol loop (serde_json over stdin/stdout lines) — no SDK dep.
3. **Backend (ajna-verify-backend)**:
   - Country Rules Engine: `rules/` module, embedded JSON rulepacks keyed by
     ISO 3166-1 alpha-2 + DEFAULT fallback; applied in /v1/verify based on
     `issuing_country`; outcome recorded in response + audit log.
   - SOC2 audit: migration 003 adds `prev_hash`/`entry_hash` (SHA-256 chain)
     + Postgres trigger blocking UPDATE/DELETE (append-only, tamper-evident);
     `GET /v1/audit/verify-chain` integrity endpoint. New queries use runtime
     sqlx (no offline-cache regeneration needed on this machine).
   - NQM compliance: verify responses carry `nqm_compliance` envelope
     (profile, algo, hybrid flag) and are server-signed via ajna-crypto
     ML-DSA when pqc enabled; crypto agility = existing algorithm negotiation.
4. **Dashboard**: React 18 + Vite + TS in `dashboard/` (replaces static
   index.html; dark theme ported). Pages: 60-min Onboarding wizard, UI
   Customizer (edits declarative UiConfig JSON with live preview), Audit Log
   viewer, API Keys. No UI framework dep beyond react.
5. **Hardware discipline**: every cargo invocation uses `-j 2`; builds strictly
   sequential; no parallel subagent builds. FFI leak check via macOS
   `leaks --atExit` (valgrind unavailable on darwin).

## Task List

### Phase 1 — Rebrand & workspace refactor (condition 1)
- [ ] git mv crates/beam-crypto → crates/ajna-crypto; rename packages
      (ajna-core, ajna-crypto, ajna-verify-backend, ajna-core-fuzz)
- [ ] Repo-wide symbol rename: beam_* → ajna_*, BeamSDK → AjnaSDK, libbeam →
      libajna, Beam → Ajna (code, header, CMake, CI, docs, SBOM; archives excluded)
- [ ] git mv renamed files (include/ajna_ffi.h, platform/*, samples com.ajna path)
- [ ] Workspace Cargo.toml: members += ajna-idv, ajna-intel, ajna-vision,
      ajna-mcp-server
- [ ] Gate: `cargo check -j 2` clean

### Phase 2 — Pillar crates (condition 1, 6)
- [ ] ajna-core: `ui_config.rs` — UiConfig (colors, overlay, animations,
      branding, strings) + UiMode::{Default, Custom, Headless} + serde +
      validation + FFI surface (JSON in/out) (condition 6)
- [ ] ajna-idv: IdvSession facade, headless runner, UiConfig re-export + tests
- [ ] ajna-intel: DeviceIndicators → PostureReport w/ risk score, signed; tests
- [ ] ajna-vision: LivenessSession FSM, cosine face match, signed result; tests
- [ ] Gate: `cargo test -p ajna-idv -p ajna-intel -p ajna-vision -j 2`

### Phase 3 — MCP server (condition 3)
- [ ] ajna-mcp-server: JSON-RPC loop, initialize/tools/list/tools/call,
      4 tools wired to ajna-intel/ajna-vision locally + backend REST
- [ ] Dispatcher unit tests (initialize handshake, tool schemas, tool calls)

### Phase 4 — Backend (conditions 2, 4)
- [ ] rules/ module: CountryRulePack, embedded rulepacks (IN, US, GB, DE, BR,
      NG, SG, AE + DEFAULT), resolve(), applied in /v1/verify + tests
- [ ] Migration 003: audit chain columns + append-only trigger
- [ ] AuditLog model: SHA-256 hash chaining on insert; /v1/audit/verify-chain
- [ ] NQM envelope + server-side response signing via ajna-crypto
- [ ] Gate: `cargo test -p ajna-verify-backend -j 2` (SQLX_OFFLINE=true)

### Phase 5 — Dashboard (condition 5)
- [ ] Vite + React + TS scaffold in dashboard/; port dark theme
- [ ] Onboarding wizard (key → platform → snippet → test call → live)
- [ ] UI Customizer bound to declarative UiConfig schema (condition 6 surface)
- [ ] Audit viewer + API keys pages
- [ ] Gate: `npm run build` passes

### Phase 6 — FFI & final verification (condition 7)
- [ ] include/ajna_ffi.h + tests/ffi_integration_tests.cpp updated to ajna_* API
- [ ] Build ajna-core staticlib (host), compile & run FFI tests,
      `leaks --atExit` → 0 leaks
- [ ] `RUSTFLAGS="-D warnings" cargo test --release -j 2` — full workspace green
- [ ] cargo fmt --check + clippy clean
- [ ] README.md updated to Ajna platform architecture
- [ ] Review section appended below

## Review

(to be completed at the end)
