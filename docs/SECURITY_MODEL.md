# Beam SDK Security Model

## Post-Quantum Cryptography

### Why ML-DSA Replaces ECDSA

Beam SDK signs every `ScanResult` with **ML-DSA** (CRYSTALS-Dilithium, FIPS 204) rather than classical ECDSA for one reason: the **harvest-now, decrypt-later threat**.

Surt's identity verification data carries long-term compliance value. An adversary who stores signed result blobs today can decrypt them once a Cryptographically Relevant Quantum Computer (CRQC) becomes available — estimated window: **2030–2040** per NIST IR 8547 (2024). ECDSA signatures are broken by Shor's algorithm on a sufficiently powerful quantum computer.

ML-DSA is lattice-based and provides **128-bit post-quantum security** at Level 3 (Dilithium-3). It is computationally infeasible to forge under both classical and quantum attack models.

### NIST Standardisation Timeline

| Standard | Algorithm | Published |
|----------|-----------|-----------|
| FIPS 203 | ML-KEM (Kyber) | August 2024 |
| FIPS 204 | ML-DSA (Dilithium) | August 2024 |
| FIPS 205 | SLH-DSA (SPHINCS+) | August 2024 |

Beam uses FIPS 203 and FIPS 204. FIPS 205 is not currently used but is listed for completeness.

---

## Key Storage Per Platform

| Platform | Key Storage | Notes |
|----------|-------------|-------|
| iOS | **Secure Enclave** (SEP) | Private key never leaves the enclave. Beam calls the Security framework to request a sign operation; the raw key bytes are not accessible to application code. |
| Android | **StrongBox Keymaster** (API 28+) | Hardware-backed key storage via Android Keystore. Falls back to TEE-backed storage on devices without StrongBox. |
| WASM | **In-memory only** | Private key bytes live in WASM heap for the duration of the session. **This is a documented limitation.** There is no browser equivalent to Secure Enclave. Integrators should treat WASM-signed results as lower assurance and enforce server-side re-verification. |

---

## Hybrid Classical + PQC Transition Recommendation (2025–2028)

During the transition period, Surt recommends a **hybrid signature scheme**:

1. Sign with both ECDSA P-256 and ML-DSA Level 3.
2. Include both signatures in `ScanResult.pqc_signature` (wrapped in a container).
3. Verifying parties accept either signature until CRQC threat materialises.
4. After 2028: deprecate ECDSA; enforce ML-DSA-only verification.

This preserves backward compatibility with classical verifiers while establishing quantum-safe signatures now.

---

## Transport Security

- **Session key**: ML-KEM-1024 (Kyber-1024) encapsulation before transmission. Ciphertext: 1568 bytes. Shared secret: 32 bytes (AES-256-GCM key material).
- **Transport**: TLS 1.3 minimum. TLS 1.2 is not permitted by the Beam server.
- **Certificate pinning**: Integrators MUST pin the Surt API certificate. Use `NSURLSession` trust evaluation on iOS and `OkHttp CertificatePinner` on Android.
- **Result encryption**: `ScanResult` payload is encrypted with AES-256-GCM using the ML-KEM shared secret before TLS transmission.

---

## Attack Surface Inventory

### 1. Adversarial documents
- **Threat**: Physical or synthetic documents designed to fool the OCR/ML layer.
- **Mitigation**: Quality gates reject frames below confidence threshold. PQC signature is over the extracted fields, not the raw image — a low-confidence result is flagged explicitly via `ScanResult.confidence`.

### 2. Camera feed injection
- **Threat**: Malicious app overlays a pre-recorded document frame onto the camera stream.
- **Mitigation**: Motion gate (`motion_score > 0.12` rejects static/replayed frames). Liveness detection is a higher-level concern outside Beam's scope; implement at the application layer.

### 3. Gralloc buffer pool starvation (Android)
- **Threat**: Rapid acquisition of `ImageReader` buffers without releasing them causes pipeline stall.
- **Mitigation**: `BeamCameraAdapter` uses `acquireLatestImage()` and calls `image.close()` unconditionally. Buffer count is capped at 4 (HAL pipeline depth on Helio G85 + 1).

### 4. JNI boundary misuse (Android)
- **Threat**: Passing NULL or stale pointers to native functions causes native crash.
- **Mitigation**: All `#[no_mangle]` functions in `ffi.rs` perform null checks on handles. JNI handles are validated before use. `BeamNativeBridge.kt` documents lifetime requirements for all `Long` handle parameters.

### 5. mlock bypass
- **Threat**: OS swap of private key pages exposes key material in swap partition.
- **Mitigation**: `PqcSigner::generate()` calls `libc::mlock()` on the private key Vec on non-WASM targets. The key is zeroed (volatile write) and `munlock()`-ed in `Drop`. On WASM, this protection is unavailable — see Platform Key Storage above.

### 6. Signature replay
- **Threat**: Attacker re-submits a previously signed `ScanResult` for a different user.
- **Mitigation**: `canonical_bytes()` encodes document fields deterministically. The server must enforce nonce-or-timestamp freshness on received results. Beam does not include a nonce in the signature payload by design — this is a server responsibility.
