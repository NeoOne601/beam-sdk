# RESEARCH.md — Standards, Threat & Market Research

> Scorecard file (CLAUDE.md §3). Facts vs assumptions separated per §14; every entry
> carries a confidence level. Entries below knowledge-cutoff quality — items marked
> [VERIFY] are queued for a web-research cycle. Last updated: 2026-07-12.

## Standards & Regulation

| Finding | Source class | Confidence | Implication for Ajna |
|---|---|---|---|
| FIPS 204 (ML-DSA) and FIPS 203 (ML-KEM) finalized Aug 2024 | NIST | HIGH | Our ML-DSA-65 + ML-KEM choice is standards-final. Gap: `pqcrypto-dilithium` is PQClean Round-3 (3309 B sigs vs FIPS-final 3293 B) — upgrade path documented in tests. [VERIFY latest FIPS-final Rust crates, e.g. `ml-dsa` (RustCrypto) / `fips204`] |
| NIST SP 800-63-4 digital identity guidelines (rev 4) | NIST | MEDIUM [VERIFY final status/date] | Align IAL/AAL terminology in docs; injection-attack resistance for biometrics is explicitly called out — supports our anti-injection roadmap |
| eIDAS 2.0 (EU 2024/1183) in force; EUDI Wallet rollout by ~2026–27 | EU | HIGH | Wallet-based, holder-centric identity is the direction of travel — plan a verifiable-credential issuance/verification interface (Horizon 3) |
| India DPDP Act 2023; DPDP Rules operationalization ongoing | MeitY | MEDIUM [VERIFY rules status 2026] | On-device processing (PII never leaves device) is a strong DPDP story; document data-fiduciary mapping |
| India National Quantum Mission — PQC migration guidance | NQM/MeitY | MEDIUM [VERIFY specific crypto mandates] | Crypto-agility registry (ADR-001) + algorithm negotiation endpoint is the right shape; keep hybrid classical+PQC on roadmap |
| ISO/IEC 30107-3 — biometric presentation attack detection (PAD) levels | ISO | HIGH | Target: document a PAD Level 1/2 test plan; incumbents (iProov) certify against this — our credibility gap |
| C2PA content provenance (2.x) | C2PA | HIGH | Capture-time attestation of camera frames is adjacent to our signed-result envelope; natural Horizon-3 extension |
| FIDO2 / WebAuthn passkeys mainstream | FIDO | HIGH | Not a competitor — a complement: passkeys authenticate returning users; Ajna proves the human at enrollment. Positioning point. |

## AI Fraud Threat Landscape

| Threat | Assessment | Confidence | Our posture |
|---|---|---|---|
| Deepfake face swaps at onboarding | Now commodity (open-source real-time swaps) | HIGH | Challenge-response FSM + motion SAD check helps vs replays; **weak vs injection** — see below |
| Virtual-camera / frame-injection attacks | Fastest-growing IDV attack class; bypasses "what the camera sees" entirely | HIGH | Roadmap: capture-path attestation (device attestation + Intel posture already detects hooks/emulators; bind camera provenance into signed envelope) |
| Synthetic/forged documents (GenAI) | Template-perfect fakes defeat visual inspection | HIGH | Checksum-validated fields (Verhoeff/ICAO/AAMVA) catch naive fakes; NFC chip read (passport/eID) is the durable answer — roadmap |
| Agent-run fraud farms | Emerging: LLM agents driving thousands of onboarding attempts | MEDIUM | Rate limiting per tenant exists; velocity/behavioral signals are a product gap and a data moat opportunity |

## Working Assumptions (testable)

1. PQC compliance becomes a procurement checkbox in India (NQM) before the West — wedge holds. [monitor]
2. Agent-to-service verification (MCP-style tool calls) becomes a standard integration surface by 2028 — our MCP server is early, not wrong.
3. Liveness without injection-resistance will be near-worthless by ~2028; PAD certification + capture attestation is existential, not optional.

## Research Queue (next research cycles)

- [ ] Verify FIPS-final ML-DSA Rust crate maturity; plan migration off PQClean Round-3.
- [ ] SP 800-63-4 final text — map IAL2/IAL3 evidence requirements to Ajna flows.
- [ ] DPDP Rules 2026 status + consent-manager ecosystem (India Stack) integration.
- [ ] ISO 30107-3 PAD test-plan design (can we self-evaluate before paying a lab?).
- [ ] Competitor teardown with citations (pricing pages, docs) → deepen PRODUCT.md matrix.
- [ ] C2PA capture-attestation prototype scope.

## Success Criterion Mapping (§12)

- Criterion 4 (AI fraud strategy): threat table above + ROADMAP items. Status: **partial — needs the injection-resistance design doc**.
- Criterion 5 (PQC migration strategy): ADR-001 + FIPS rows above + hybrid plan on roadmap. Status: **partial — hybrid design doc pending**.
- Criterion 12 (regulatory analysis): table above. Status: **YES (v1; deepen with [VERIFY] items)**.
