# Beam Verify SDK — Threat Model

## Document Purpose

This document catalogues security threats against the Beam Verify SDK and its backend verification service, maps each to implemented controls, and identifies residual risks.

## Scope

- On-device Beam Verify SDK (Rust core, C++ ML bridges, platform adapters)
- Beam Verify backend service (Axum, PostgreSQL, Redis)
- Communication between SDK and backend

## Out of Scope

- Face biometric systems (FaceGuard — separate product)
- Server-side entity graph construction
- Third-party integrator application security
- Physical document fraud detection beyond on-device ML
- NFC chip reading (Phase 2 roadmap)

---

## Threat Categories

### T1: Adversarial Document Injection

**Threat**: Physical or synthetic documents designed to fool the OCR/ML pipeline. Includes screen photos, printed fakes, and adversarial patches.

**Controls**:
| Control | Implementation | Residual Risk |
|---------|----------------|---------------|
| Quality gates | 4-stage CPU-only pipeline rejects low-quality frames before inference | Medium — sophisticated physical fakes pass quality gates |
| Fraud signals | `is_screen_photo` and `is_printed_fake` outputs in model schema | Medium — model accuracy depends on training data coverage |
| Confidence thresholds | Minimum 0.85 confidence before result is accepted | Low — threshold may reject legitimate difficult documents |
| MRZ checksum | ICAO 9303 check digit validation in `field_parser.rs` | Low — only validates MRZ internal consistency |

### T2: Camera Feed Manipulation

**Threat**: Malicious app or rooted device overlays pre-recorded/synthetic frames onto the camera stream.

**Controls**:
| Control | Implementation | Residual Risk |
|---------|----------------|---------------|
| Motion gate | Inter-frame SAD rejects static/replayed frames | Medium — sophisticated replay with slight motion may pass |
| Frame timestamps | Monotonic clock timestamps validated in session state machine | Low — requires kernel-level compromise to spoof |

**Residual Risk**: On rooted/jailbroken devices, the camera HAL can be fully compromised. Liveness detection (FaceGuard) is recommended as an additional layer.

### T3: Private Key Extraction

**Threat**: Attacker extracts the ML-DSA private key from device memory.

**Controls**:
| Control | Implementation | Residual Risk |
|---------|----------------|---------------|
| mlock() | Private key pages locked in RAM via `libc::mlock()` on non-WASM | Low — prevents swap exposure |
| Volatile zeroing | Key zeroed via volatile write in `Drop` implementation | Low — prevents heap reuse exposure |
| WASM documented limitation | WASM keys are in-memory only; documented as lower assurance | High — browser sandbox provides no OS-level memory protection |

**Residual Risk**: A compromised OS kernel can read any process memory regardless of mlock(). Hardware security element integration (Secure Enclave / StrongBox) is on the Phase 2 roadmap and will eliminate this residual risk for iOS and Android.

### T4: Signature Replay / Result Tampering

**Threat**: Attacker intercepts a signed `ScanResult` and replays it for a different identity verification session, or tampers with the result payload.

**Controls**:
| Control | Implementation | Residual Risk |
|---------|----------------|---------------|
| ML-DSA signature | `ScanResult::canonical_bytes()` signed with Dilithium-3 | Very Low — post-quantum secure against forgery |
| Nonce-protected verification | Backend issues single-use nonces (Redis, TTL 300s) | Low — nonce must be consumed within 5 minutes |
| Deterministic encoding | `canonical_bytes()` produces identical output for identical fields | None — tampered fields produce invalid signatures |
| TLS 1.3 transport | Minimum TLS 1.3 for backend communication | Low — standard TLS threat model applies |

### T5: Backend Infrastructure Compromise

**Threat**: Attacker gains access to the backend database, Redis instance, or API.

**Controls**:
| Control | Implementation | Residual Risk |
|---------|----------------|---------------|
| Input validation | All endpoints validate request structure and types | Low |
| SQL parameterisation | sqlx uses parameterised queries exclusively | Very Low — no SQL injection vector |
| Webhook HMAC | Webhook payloads signed with HMAC-SHA256 | Low — secret compromise requires DB access |
| Audit logging | All verification events logged with IP address | None — forensic capability only |

### T6: Supply Chain / Dependency Compromise

**Threat**: A dependency (pqcrypto-dilithium, TFLite, ONNX Runtime) is compromised upstream.

**Controls**:
| Control | Implementation | Residual Risk |
|---------|----------------|---------------|
| SBOM | CycloneDX 1.4 SBOM in `compliance/SBOM.json` | None — tracking capability only |
| Model signing | Ed25519 detached signatures on model artifacts | Low — requires signing key compromise |
| Cargo.lock | Exact dependency versions pinned | Low — requires supply chain attack on crates.io |
| CI verification | Model signatures verified in CI before packaging | Low |

---

## Risk Summary Matrix

| Threat | Likelihood | Impact | Controls | Residual |
|--------|-----------|--------|----------|----------|
| T1: Adversarial documents | Medium | High | Quality gates, fraud signals, MRZ validation | Medium |
| T2: Camera manipulation | Low | High | Motion gate, timestamps | Medium |
| T3: Key extraction | Low | Critical | mlock(), volatile zero, Phase 2 HSE | Medium (WASM: High) |
| T4: Signature replay | Low | High | ML-DSA, nonces, canonical encoding | Low |
| T5: Backend compromise | Low | High | Input validation, parameterised SQL, HMAC | Low |
| T6: Supply chain | Very Low | Critical | SBOM, model signing, Cargo.lock | Low |
