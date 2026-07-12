# DECISIONS.md — Architecture Decision Records

> Scorecard file (CLAUDE.md §3). Format: context → decision → alternatives → outcome.
> Pre-cycle decisions are summarized from README.md ("Architecture Decision Log" +
> VR-1..VR-6 remediation log) so this file is the single ADR index going forward.

## ADR-001 — Cryptographic agility via SignerRegistry (accepted, pre-cycle)
- **Context:** NQM requires dynamic negotiation of ML-DSA and classical algorithms.
- **Decision:** `ajna-crypto` crate with a thread-safe `SignerRegistry`; Ed25519 +
  ML-DSA-65 implemented; ECDSA/hybrid stubbed. `POST /v1/session/init` negotiates.
- **Alternatives:** hard-coded single algorithm (fails NQM); linking liboqs (heavier).
- **Outcome:** shipped, tested.

## ADR-002 — Rust core, C++ only at ML runtime boundary (accepted, pre-cycle)
- See README "Architecture Decision Log" for the full rationale set (Rust vs Go/C++,
  NV12 native, ordered quality gates, 25 fps lock, 4-buffer pool, PQClean Round-3).

## ADR-003 — Trusted key registration, strategy pattern (accepted, pre-cycle, VR-2)
- **Decision:** `KeyProvider` trait — tenant | device | model, env-selected.
- **Outcome:** shipped; enterprise migration path without code change.

## ADR-004 — SOC2 audit as append-only SHA-256 hash chain in Postgres (accepted, pre-cycle)
- **Decision:** PL/pgSQL trigger chains every audit row; `/v1/audit/verify-chain`
  proves integrity. **Outcome:** shipped, deployed.

## ADR-005 — MCP server as first-class product surface (accepted, pre-cycle)
- **Decision:** stdio JSON-RPC MCP server exposing 4 verification tools so AI agents
  consume Ajna directly. **Outcome:** shipped, smoke-tested; unique vs competitors.

## ADR-006 — Dashboard enterprise stack (accepted, 2026-07-12, cycle 1)
- **Context:** §15 — dashboard is school-project grade; must become an enterprise
  console without losing the tactical HUD brand.
- **Decision:** `react-router-dom` (v7 as installed) (D1), `recharts` (D4), `lucide-react` (D6);
  in-house toasts, command palette, auth context, telemetry polling hook; extend
  `theme.css` — no Tailwind, no component library.
- **Alternatives:** Tailwind/MUI/Ant (rejected — §15 explicitly forbids; the custom
  CSS is the brand differentiator); WebSockets for telemetry (rejected for now —
  polling is sufficient at demo scale and works against the sleeping free-tier
  backend; upgrade path documented in code).
- **Approval note:** §13 gates new npm deps; these three are explicitly enumerated in
  §15 "Technology Decisions", treated as pre-approved by the rulebook. No other deps added.
- **Outcome:** implemented cycle 1.

## ADR-007 — Demo authentication is client-side (accepted, 2026-07-12, cycle 1)
- **Context:** D2 requires a login gate with demo credentials + JWT/session tokens.
  The backend has real API-key/JWT auth for its API, but no portal-user identity
  system (no user table, no password hashes) — building one is a backend feature
  with security surface (password storage) that §13 would gate before production.
- **Decision:** Portal ships a demo auth gate: demo credentials validated client-side,
  a demo JWT (HS256-shaped, unsigned-verifiable, clearly labeled) stored in
  `sessionStorage`, `RequireAuth` route guard. Clearly marked "demo tenant".
- **Alternatives:** real portal-user auth in backend (deferred — needs schema +
  password hashing + reset flows; roadmap Phase 2); OAuth (needs a provider — Human Gate).
- **Consequence:** D2 satisfied for the integration-portal demo; production portal
  auth is an explicit ROADMAP item, not silently missing.

## ADR-008 — 10–15 year platform thesis: agentic trust substrate (proposed)
- **Context:** /goal mission — optimize for the market 10–15 years out.
- **Direction:** evolve from "IDV vendor" to "trust infrastructure": (1) capture-time
  provenance (C2PA-adjacent) vs injection attacks, (2) verifiable-credential interface
  (eIDAS 2.0 wallets), (3) AI-agent identity + scoped delegation chains signed with
  the same PQC envelope, (4) audit chain as compliance-evidence product.
- **Status:** proposed — validate via research cycles (RESEARCH.md queue) before
  committing engineering resources. No conventional-IDV work is discarded.

## ADR-009 — VR-2 enforcement + fail-closed JWT (accepted, 2026-07-12, cycle 3)
- **Context:** independent audit (§9 verifier) found VR-2 was documented but not
  enforced — both verification paths trusted client-supplied public keys; the
  registered-key lookup was only an existence check. Separately, unset
  `JWT_SECRET` silently accepted **unsigned** bearer tokens.
- **Decision:** (1) `/v1/verify` now verifies exclusively against the registered
  key; client-supplied key material must match byte-for-byte or the request is
  401-rejected. Demo escape hatch `ALLOW_UNREGISTERED_ED25519_KEYS=true`
  (default false) permits ed25519 fallback, warned in logs and recorded as
  `key_trust: "client-supplied-demo"` in the audit chain. (2) Bearer tokens are
  rejected outright when `JWT_SECRET` is unset (API-key auth unaffected).
  (3) `verification_results.pqc_public_key_hex` now stores the hex of the key
  actually used (was: hex of the base64 signature string — wrong content).
- **Alternatives:** strict-only with no demo flag (breaks the reference demo,
  whose signer uses ephemeral keys); trust-on-first-use (silently converts an
  attacker's first key into "the" key — rejected).
- **Outcome:** backend tests green (37/37), clippy clean; deploy docs updated.

## Risk register (criterion 10)

| Risk | Status |
|---|---|
| PQClean Round-3 vs FIPS-final ML-DSA discrepancy | Open, documented; migration queued (RESEARCH.md) |
| Liveness vs virtual-camera injection | Open; ADR-008 direction (1); design doc queued |
| Portal auth is demo-grade | Accepted for demo (ADR-007); Phase 2 roadmap item |
| Free-tier backend cold start (~50 s) skews demos | Mitigated: dashboard telemetry falls back to simulator |
| Tenant API keys stored/compared in plaintext (`tenants.api_key`) | Open — hash-at-rest migration queued Phase 2 (needs key-issuance flow changes) |
| `/v1/webhooks` validates but does not persist; delivery unwired | Disclosed in openapi.yaml; wire-up or removal queued Phase 2 |
| SSRF check is string-level (no DNS resolution) | Low exposure while delivery is unwired; resolve-time check queued with webhook wire-up |
| Signed JWTs skip `exp` validation (`validate_exp=false`) | Open — enable once issued tokens carry `exp`; documented in auth.rs |
