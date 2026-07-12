# Ajna AI Fraud Strategy

> §12 criterion 4 evidence: anti-deepfake, anti-injection, and liveness
> anti-spoofing posture — what exists today, where the gaps are, and the
> committed design direction. Confidence levels per CLAUDE.md §14.
> Last updated: 2026-07-12 (cycle 2). Companion: RESEARCH.md threat table,
> docs/SECURITY_MODEL.md.

## Threat taxonomy (ranked by expected 2026–2030 prevalence)

| # | Attack | Vector | Prevalence trend |
|---|---|---|---|
| T1 | Presentation attacks (printed photo, screen replay, masks) | Camera sees a fake | Flat — commodity defense exists industry-wide |
| T2 | Virtual-camera / frame injection | Camera is bypassed entirely; synthetic frames fed to the app | **Fastest-growing** (HIGH confidence) — defeats any purely visual liveness |
| T3 | Real-time deepfake face swap | T1 or T2 delivery of a GAN/diffusion face | Rising fast; free tooling |
| T4 | Synthetic/forged documents | GenAI-rendered template-perfect IDs | Rising; visual inspection already defeated |
| T5 | Agent-run fraud farms | LLM agents mass-driving onboarding | Emerging |

## Layered defense — what exists today (shipped, tested)

| Layer | Mechanism | Counters | Where |
|---|---|---|---|
| Frame forensics | MotionCheck — normalized SAD > 0.12 rejects static/replayed frames | T1 (screen/print replay) | `core/src/quality.rs` |
| Challenge-response liveness | FSM with randomized blink/smile/turn prompts, attempt budgets, wall-clock timeout, anti-replay | T1, naive T3 (pre-recorded swaps can't follow random prompts) | `crates/ajna-vision/src/liveness.rs` |
| Landmark geometry | EAR/MAR/Yaw derived from MediaPipe FaceMesh — gesture must be geometrically consistent | T3 (low-fidelity swaps break geometry) | `crates/ajna-vision/src/landmarks.rs` |
| Device posture | Root/jailbreak artifacts, hooking frameworks (Frida/Xposed), emulator property detection → weighted, signed `PostureReport` | **T2 partially** — most injection tooling requires root/hooks/emulators | `crates/ajna-intel/src/checks.rs` |
| Document validation | Checksum-validated fields: Aadhaar Verhoeff, passport ICAO MRZ check digits, AAMVA parsing | T4 (naive fakes fail checksums) | `crates/ajna-idv/src/ocr.rs` |
| Cryptographic capture binding | Result signed on-device (ML-DSA-65) with nonce + session + timestamp bound into canonical bytes (VR-1); keys pre-registered server-side (VR-2) | Post-capture tampering, replay, result forgery | `core/src/crypto.rs`, backend |
| Rate limiting | Redis token-bucket per tenant | T5 (blunt) | backend middleware |

**Honest assessment:** the stack is strong against T1/T4 and post-capture
tampering, moderate against T3, and **structurally weak against T2** when the
attacker controls a clean (non-rooted) device with an OS-level virtual camera
— the same gap almost every IDV vendor has (MEDIUM confidence; iProov-class
vendors mitigate with server-side flash-response challenges).

## Committed design direction (ADR-008 direction 1)

### D-1: Capture-path attestation (T2 — the priority)
Bind *how the frames were captured* into the signed envelope, not just what
they contained:
1. **Platform camera provenance** — iOS: require frames sourced from an
   `AVCaptureSession` the SDK itself owns; Android: verify `CameraCharacteristics`
   consistency + Play Integrity / Key Attestation verdict as a `DeviceIndicators`
   input. Emit `capture_source` into `ScanResult` fields so it is PQC-signed.
2. **Posture gating** — verification fails closed when `PostureReport` shows
   hooking frameworks or emulator markers and the tenant's country rulepack
   demands hardware capture (extend `CountryRulePack` with `require_attested_capture`).
3. **Server cross-check** — backend rejects results whose signed
   `capture_source` field is absent when the rulepack requires it (one new
   rule in `rules/mod.rs`; no schema break — fields are open key/value).

Status: design committed, implementation queued (ROADMAP Phase 2). Confidence
HIGH that this closes the commodity (non-kernel) injection tier; kernel-level
injection on rooted devices remains detectable only via posture signals.

### D-2: Active-illumination liveness (T2/T3 hardening, research)
Randomized screen-color flash sequences reflected off the face, validated
server-side against the challenge seed — injection must synthesize correct
reflections in real time. Research item (RESEARCH.md queue): patent landscape
(iProov holds material prior art here — zero-plagiarism rule §16 applies;
design must be independently derived or licensed). Confidence MEDIUM.

### D-3: PAD evidence (ISO/IEC 30107-3)
Self-evaluation harness first (attack-species corpus: print, replay, 2D mask),
accredited-lab Level 1 test when budget allows (Human Gate §13 — spend).
Status: queued.

### D-4: Velocity + behavioral signals (T5)
Tenant-level anomaly signals from the audit chain we already write (attempt
velocity per device/key, geo dispersion, time-of-day entropy). Data exists;
scoring job queued Phase 2. Confidence HIGH (cheap, high signal).

## Non-goals
- No silent PII-hungry biometric databases — signals stay on-device or signed-summary only (DPDP/GDPR posture, PRODUCT.md).
- No claim of "deepfake-proof". Marketing language must match this document.

## Criterion 4 verdict
Documented with evidence: current mechanisms (shipped + file references),
gap analysis (T2), committed designs (D-1..D-4) with confidence levels and
standards mapping (ISO 30107-3, Play Integrity, FIPS 204 binding). Status:
**YES (v1)** — deepen D-2 patent research in a research cycle.
