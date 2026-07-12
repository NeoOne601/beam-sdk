# ROADMAP.md — Phased Roadmap

> Scorecard file (CLAUDE.md §3). Last updated: 2026-07-12 (cycle 1).

## Phase 0 — Foundation (DONE, pre-cycle)
Rust core + crypto + 3 pillars + MCP server + backend (VR-1..6 hardened) + reference
deployment + mobile SDK packaging. ~146 tests green.

## Phase 1 — Enterprise credibility (CURRENT)
| Item | Criterion | Status |
|---|---|---|
| Dashboard enterprise overhaul (D1–D14) | §12.13/14 | **Done cycle 1** — verifier PASS (visual: PREMIUM); LOW items carried in tasks/todo.md |
| State docs bootstrapped (PRODUCT/ARCHITECTURE/RESEARCH/DECISIONS/ROADMAP) | §12.1,2,10,11,12 | Cycle 1 — done |
| OpenAPI spec for backend endpoints | §12.7 | Queued |
| AI-fraud (anti-injection) strategy doc | §12.4 | Queued (RESEARCH.md queue) |
| PQC hybrid + FIPS-final migration design | §12.5 | Queued |
| Identity model consolidation w/ sequence diagrams | §12.6 | Queued |
| Docs freshness sweep (no stale references) | §12.9 | Queued |
| On-device field testing (blocked: NDK/SDK installs — Human Gate §13) | — | Blocked on owner |

## Phase 2 — Production hardening
Real portal-user auth (replaces ADR-007 demo gate) · device-level key rollout ·
PAD (ISO 30107-3) self-evaluation plan · NFC chip-read (passport/Aadhaar offline XML) ·
velocity/behavioral fraud signals · WebSocket telemetry · usage metering → billing.

## Horizon 3 — Agentic trust substrate (ADR-008, validate first)
Capture-time provenance (C2PA-adjacent) · verifiable-credential issue/verify (eIDAS 2.0
wallet interop) · AI-agent identity + scoped delegation chains (PQC-signed) · audit
chain as a compliance-evidence product.

## Dependencies & gates
- Mobile field testing → owner installs NDK/SDK/emscripten (§13).
- Pricing publication → business approval (§13).
- Production portal auth → backend schema work (Phase 2).
