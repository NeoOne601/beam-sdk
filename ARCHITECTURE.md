# ARCHITECTURE.md — Ajna System Architecture

> Scorecard file (CLAUDE.md §3). Deep detail lives in `README.md` (primary design doc)
> and `docs/` — this file is the navigable summary + topology. Last updated: 2026-07-12.

## Components

| Component | Path | Language | Role |
|---|---|---|---|
| Scanning engine | `core/` | Rust | Quality gates (ordered short-circuit), session FSM, `UiConfig`, PQC signing, C FFI (`include/ajna_ffi.h`) |
| Crypto foundation | `crates/ajna-crypto` | Rust | `SignerRegistry` — Ed25519 + ML-DSA-65 (FIPS 204); ECDSA/hybrid stubbed (ADR-001) |
| IDV pillar | `crates/ajna-idv` | Rust | Headless scanner; OCR parsers — Aadhaar (Verhoeff), Passport (ICAO MRZ), US DL (AAMVA) |
| Intel pillar | `crates/ajna-intel` | Rust | Device posture: root/jailbreak/hook/emulator indicators → weighted, signed `PostureReport` |
| Vision pillar | `crates/ajna-vision` | Rust | Challenge-response liveness FSM, EAR/MAR/Yaw landmark geometry, anti-replay |
| MCP server | `crates/ajna-mcp-server` | Rust | stdio JSON-RPC 2.0; 4 agent tools (posture, face, document, audit) |
| Verification backend | `backend/` | Rust (Axum) | `/v1/nonce`, `/v1/verify`, `/v1/audit`, `/v1/webhooks`, `/v1/session/init`; country rules engine; NQM attestation; SOC2 hash-chained audit log |
| Integration portal | `dashboard/` | React/Vite TS | Enterprise console: auth, telemetry, onboarding, UI customizer, audit viewer, API keys (§15 overhaul — cycle 1) |
| Platform adapters | `platform/{ios,android,wasm}` | Swift/Kotlin/C++ | Thin camera/ML bridges — zero business logic |
| Samples | `samples/{ios,android,web}` | — | Reference apps wiring real cameras + on-device ML |

## Data Flow (verification happy path)

```
Camera frame (NV12, zero-copy)
  → core quality gates (luma → blur → motion → ML, ordered short-circuit)
  → session FSM (25 fps budget, wall-clock timeout)
  → pillar logic (IDV OCR / Vision liveness / Intel posture)
  → ScanResult.canonical_bytes()  [+ __nonce, __session_id, __timestamp — VR-1]
  → ML-DSA-65 edge signature (ephemeral key, mlock'd, zeroed on Drop — VR-6)
  → POST /v1/verify (X-Api-Key or JWT → TenantContext — VR-3)
  → KeyProvider strategy (tenant | device | model — VR-2) → signature verify
  → country rules engine (ISO resolution, NQM enforcement)
  → append-only SHA-256 hash-chained audit log (Postgres trigger, SOC2)
  → signed NQM server attestation → response / webhook (SSRF-guarded — VR-5)
```

## Trust Boundaries

1. **FFI boundary** (`core/src/ffi.rs`): all pointers null-checked, dims validated,
   `i32` status codes (VR-4). Leak-checked to 0 bytes.
2. **Device → backend**: nothing client-supplied is trusted; keys are pre-registered
   (VR-2), nonces bound into signed bytes (VR-1), tenant-scoped everything (VR-3).
3. **Backend → outbound**: webhook URLs parsed and RFC-1918/metadata-blocked (VR-5).
4. **Dashboard**: browser-side only; never persists secret key material; talks to
   backend with the tenant's key. Demo auth is client-side (documented as demo).

## Deployment Topology

| Mode | Stack | Notes |
|---|---|---|
| Local dev | docker-compose: backend + dashboard + Postgres (256 MB) + Redis (128 MB) | 8 GB M1 memory caps are mandatory |
| Reference ($0) | Render (Axum Docker) + Supabase Postgres + Render/Upstash Redis + Vercel (dashboard) | env-driven; free tier cold-starts ~50 s |
| Enterprise | Same binary; swap `DATABASE_URL`/`REDIS_URL`/`KEY_PROVIDER_STRATEGY` | No code change — decoupled by configuration |

## Dashboard architecture (post-§15 overhaul, cycle 1)

- **Routing:** `react-router-dom` v6 — `/login`, `/` (Operations overview), `/architecture`,
  `/onboarding`, `/customizer`, `/audit`, `/keys`. Auth-gated via `RequireAuth`.
- **State:** React Context + `useReducer` for auth session (demo JWT in `sessionStorage`);
  component-local state elsewhere. No global store.
- **Telemetry:** polling hook (2 s interval) — tries the live backend, falls back to a
  deterministic simulator so the console is always demonstrable.
- **Charts:** `recharts` (volume time-series, pass/fail donut, geo distribution).
- **Design system:** in-house tactical HUD CSS (`theme.css`) — deliberately no Tailwind
  or component library (§15 "What NOT to do"). Icons: `lucide-react`.

## Success Criterion Mapping (§12)

- Criterion 2 (production-ready architecture doc): this file + README. Status: **YES (v1)**.
- Criterion 6 (identity model + sequence diagrams): README §Architecture/§PQC + `docs/SECURITY_MODEL.md`. Status: **YES (consolidate here in a docs cycle)**.
- Criterion 7 (API docs): `docs/API_REFERENCE.md`. OpenAPI spec: **NO — roadmap item**.
