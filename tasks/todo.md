# Ajna GTM Platform — Implementation Spec & Task List

Goal: evolve the Beam edge-PQC document scanner into **Ajna**, a three-pillar
GTM security platform (IDV, Intel, Vision) per the /goal termination conditions.

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
     FFI, declarative `UiConfig` + headless mode types.
   - `ajna-crypto`: unchanged signer registry (Ed25519, ML-DSA-65, hybrid).
   - `ajna-idv`: product facade (IdvSession, HeadlessScanner, signer bootstrap).
   - `ajna-intel`: DeviceIndicators → risk-scored, PQC-signed PostureReport.
   - `ajna-vision`: liveness challenge FSM + cosine face match, PQC-signed.
   - `ajna-mcp-server`: hand-rolled stdio JSON-RPC 2.0 MCP server, 4 tools.
3. **Backend**: country rules engine (embedded JSON + env override),
   hash-chained append-only audit (migration 003 + trigger + verify-chain
   endpoint, runtime sqlx), NQM envelope + ML-DSA-65 server attestation.
4. **Dashboard**: React 18 + Vite + strict TS; onboarding wizard, UI
   customizer bound to UiConfig schema, audit viewer, API keys.
5. **Hardware discipline**: `-j 2` everywhere, sequential builds,
   `leaks --atExit` for FFI memory validation (no valgrind on darwin).

## Task List

### Phase 1 — Rebrand & workspace refactor (condition 1)
- [x] git mv crates/beam-crypto → crates/ajna-crypto; rename packages
- [x] Repo-wide protected symbol sweep (beam→ajna, surt→ajna; archives excluded)
- [x] git mv renamed files (include/ajna_ffi.h, platform/*, samples com.ajna path)
- [x] Workspace members += ajna-idv, ajna-intel, ajna-vision, ajna-mcp-server
- [x] Gate: `cargo check -j 2` clean (also fixed non-root profile warning;
      lto=thin at workspace root for 8GB host)

### Phase 2 — Pillar crates (conditions 1, 6)
- [x] ajna-core: ui_config.rs (UiMode Default/Custom/Headless, theme/overlay/
      animations/branding/strings, validation) + ajna_ui_config_validate FFI
- [x] ajna-idv: IdvSession facade, safe HeadlessScanner, signer bootstrap
- [x] ajna-intel: posture evaluation + artifact catalogs, signed reports
- [x] ajna-vision: liveness FSM (anti-replay/attempts/timeout), cosine match
- [x] Gate: all pillar tests green

### Phase 3 — MCP server (condition 3)
- [x] JSON-RPC loop: initialize/ping/tools/list/tools/call; 4 Ajna tools
- [x] 9 dispatcher unit tests + live stdio smoke test (signed posture report)

### Phase 4 — Backend (conditions 2, 4)
- [x] rules/: 8 country packs + DEFAULT, alpha-2 aliases, NQM pqc_required,
      applied in /v1/verify, AJNA_COUNTRY_RULES_PATH override
- [x] Migration 003: seq/prev_hash/entry_hash + append-only trigger
- [x] audit_log: SHA-256 chain writer + verifier; GET /v1/audit/verify-chain
- [x] nqm.rs: compliance envelope + ML-DSA-65 server attestation in response
- [x] Gate: 33 backend tests green (SQLX_OFFLINE)

### Phase 5 — Dashboard (condition 5)
- [x] Vite + React + strict TS scaffold; dark theme
- [x] Onboarding wizard (key → platform → snippet → test → live)
- [x] UI Customizer: live preview + validation mirroring Rust bounds + JSON export
- [x] Audit viewer (loads /v1/audit, runs verify-chain) + API keys page
- [x] Gate: `npm run build` passes (0 errors)

### Phase 6 — FFI & final verification (condition 7)
- [x] ajna_ffi.h + ffi_integration_tests.cpp: ajna_* API + 3 UiConfig tests
- [x] FFI binary: 11/11 pass; `leaks --atExit` → 0 leaks, 0 bytes
- [x] `RUSTFLAGS="-D warnings" cargo test --release -j 2` → 130 tests, 0 fail,
      0 warnings
- [x] cargo fmt clean; clippy --workspace --all-targets clean
- [x] README.md: Ajna platform overview (pillars, MCP, dashboard, compliance)
- [x] Review section below

## Review

**Termination conditions vs. delivered:**

1. **Rebrand + multi-crate workspace** ✔ — 8-member workspace: ajna-core,
   ajna-crypto, ajna-idv, ajna-intel, ajna-vision, ajna-mcp-server,
   ajna-verify-backend, ajna-core-fuzz. FFI symbols, header, bridges, samples,
   CI, SBOM all ajna-branded. Archives intentionally preserved.
2. **Country Rules Engine** ✔ — backend/src/rules/ with embedded JSON packs
   (IND requires ML-DSA per NQM; per-country confidence floors, doc types,
   required fields), wired into /v1/verify, outcome in response + audit.
3. **MCP server** ✔ — crates/ajna-mcp-server, verified end-to-end over stdio;
   local tools sign with ajna-crypto, backend tools proxy with X-Api-Key.
4. **SOC2 + NQM backend** ✔ — hash-chained append-only audit (DB trigger +
   verify-chain endpoint) and ML-DSA-65 server attestation on every verify.
5. **Dashboard** ✔ — dashboard/ Vite+React portal, builds clean.
6. **Headless + declarative UI config** ✔ — UiConfig in ajna-core (one schema:
   Rust ↔ FFI ↔ dashboard TS), HeadlessScanner in ajna-idv.
7. **Tests/warnings/FFI** ✔ — 130 release tests pass under -D warnings;
   11/11 C++ FFI tests; leaks reports 0 leaked bytes.

**Deliberate simplifications (ponytail ledger):**
- Audit chain appends serialize on a per-tenant Postgres advisory lock —
  upgrade to a queued writer if audit write rates ever contend.
- MCP face tool uses the default liveness challenge sequence; custom
  challenge configs can be added to the tool schema when a client asks.
- Dashboard API-keys page is demo-local; key issuance stays a backend concern.
- Country rulepacks are static config (embedded/env override); move to DB
  per-tenant overrides when the first tenant needs custom rules.

**Known environment quirks (not code defects):**
- Gate timing on this M1 host under Rosetta-free clang builds: 9.2 ms for
  1080p (CI bound 20 ms; device budget 4 ms applies to Helio G85 with NEON).
- The repo's code-review-graph PostToolUse hook is broken (`--quiet` flag
  not supported by the installed CLI) — errors on every file edit, harmless.
