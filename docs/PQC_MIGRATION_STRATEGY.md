# Ajna Post-Quantum Cryptography Migration Strategy

> §12 criterion 5 evidence: FIPS 203/204 compliance path, crypto-agility
> design, and the hybrid classical+PQC plan. Last updated: 2026-07-12
> (cycle 2). Companions: README §Post-Quantum Cryptography, docs/SECURITY_MODEL.md,
> DECISIONS.md ADR-001.

## Threat model driving the migration

Identity attestations signed today must remain verifiable and unforgeable
beyond 2035. "Harvest-now, forge-later" applies to signatures the moment a
CRQC exists; Shor breaks Ed25519/ECDSA. Hence PQC signing at the edge now,
not at some future rollover (HIGH confidence — NIST IR 8547 direction:
classical signatures disallowed after ~2035).

## Current state (shipped, tested)

| Element | State |
|---|---|
| Signature algorithms | Ed25519 (legacy default) + ML-DSA-65 (FIPS 204 Level 3) via `SignerRegistry` (`crates/ajna-crypto`) |
| Transport encapsulation | ML-KEM-1024 (FIPS 203) for key encapsulation (`core/src/crypto.rs`) |
| Negotiation | `POST /v1/session/init` — client/server agree on `ed25519`, `ml-dsa-65`, or `hybrid-ed25519-ml-dsa-65` (hybrid negotiated, not yet implemented) |
| Key protection | Per-session ephemeral keys; `mlock` best-effort with logged fallback (VR-6); volatile-write zeroing on Drop |
| Server side | ML-DSA-65 counter-attestation on every verification (`backend/src/nqm.rs`); NQM compliance envelope labels the algorithm used |
| Known gap | `pqcrypto-dilithium 0.5` is PQClean **Round-3** Dilithium-3: 3309-byte signatures vs FIPS-204-final 3293 bytes. Documented in tests; interoperable within our closed loop, not with FIPS-final verifiers |

## Migration path

### Phase A — FIPS-final ML-DSA (next crypto cycle)
1. Adopt a FIPS-204-final Rust implementation (candidates to verify in a
   research cycle: RustCrypto `ml-dsa`, `fips204` crate; selection criteria:
   test-vector conformance (NIST ACVP), no-std/mobile fitness, maintenance).
2. Register it in `SignerRegistry` as `ml-dsa-65` **v2**; keep Round-3 verifier
   available server-side for previously issued attestations (verify-only,
   never sign) — the registry pattern makes this a pure addition (ADR-001).
3. Re-run canonical-bytes signature test suite + cross-verify golden vectors.

### Phase B — Hybrid classical+PQC (defense in depth)
`hybrid-ed25519-ml-dsa-65`: sign canonical bytes with both; verification
requires **both** to pass. Envelope: two detached signatures + both public-key
references. Wire shape already reserved in `session/init` and the crypto
registry stubs. Rationale: hedges implementation bugs in young PQC code with
a battle-tested classical scheme (matches current NSA/BSI hybrid guidance —
MEDIUM confidence on exact regulatory wording, verify in research cycle).

### Phase C — Crypto-agility proof
Yearly drill: introduce a mock `algo=v-next` end-to-end (SDK sign → negotiate
→ verify → audit) to prove the platform rotates algorithms with **zero code
change for tenants** — the property NQM conformity actually demands.

## NQM (Indian National Quantum Mission) alignment

- Dynamic negotiation of ML-DSA + classical algorithms: shipped (`/v1/session/init`).
- Every verification response carries an `nqm_compliance` envelope naming the
  algorithm and PQC status, plus an ML-DSA-65 `server_attestation`: shipped.
- Rulepacks can mandate PQC per country (`require_pqc` in rules engine): shipped.
- Open item: track MeitY/NQM published migration mandates as they land
  (RESEARCH.md [VERIFY] queue) and map them to rulepack defaults.

## Rollout risks

| Risk | Mitigation |
|---|---|
| Round-3 ↔ FIPS-final interop break | Verify-only legacy registry entry; version tag in `algo` metadata |
| Mobile binary size (ML-DSA tables) | Feature-gated (`pqc` cargo feature) — already structured |
| Signature size (3.3 kB) on constrained links | Envelope already treats signatures as detached base64; no change needed |
| Young-implementation bugs | Phase B hybrid; ACVP vectors in CI |

## Criterion 5 verdict
Compliance path (Phase A), agility design (ADR-001 + registry + negotiation,
shipped), hybrid plan (Phase B, wire-reserved): **YES (v1)** — crate selection
pending the [VERIFY] research task.
