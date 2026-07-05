# Ajna SDK — Architecture & PQC Security Whitepaper

## Overview

Ajna is Ajna AI's cross-platform document scanning SDK, designed to deliver
post-quantum cryptographically signed identity verification results from native
mobile and web applications to the Ajna compliance backend.

## Architecture

### Language Boundary (Non-Negotiable)

```
┌─────────────────────────────────────────────────────────────────┐
│ Swift / Kotlin                                                  │
│   Camera session negotiation, format detection, surface delivery│
└────────────────────────────┬────────────────────────────────────┘
                             │ one CVPixelBuffer / AHardwareBuffer ref
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│ C++ (ML runtime boundary)                                       │
│   TFLite (Android), CoreML (iOS), ONNX Runtime (WASM)          │
│   AHardwareBuffer import, CVPixelBuffer pointer arithmetic      │
│   Raw pointer math at inference boundary ONLY                   │
└────────────────────────────┬────────────────────────────────────┘
                             │ exactly ONE FFI call per accepted frame
                             │ ajna_session_push_result()
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│ Rust (all business logic)                                       │
│   Quality gates, session state machine, result parsing          │
│   PQC signing (ML-DSA FIPS 204), FFI boundary declarations     │
└─────────────────────────────────────────────────────────────────┘
```

### Quality Gate Pipeline

Gates execute in strict order. The C++ GPU layer is **never** invoked on rejected frames.

```
Frame arrives
     │
     ▼
[Gate 1: Blur]  — Laplacian variance on 64×64 centre crop
     │ fail → QualityReport(gate=BlurCheck, all downstream=0.0)
     ▼
[Gate 2: Exposure] — Luma histogram: mean ∈ [40,220], P95 < 245
     │ fail → QualityReport(gate=ExposureCheck, motion=0.0, edge=0.0)
     ▼
[Gate 3: Motion] — Normalised SAD vs previous frame < 0.12
     │ fail → QualityReport(gate=MotionCheck, edge=0.0)
     ▼
[Gate 4: Boundary] — Sobel edge density > 0.08
     │ fail → QualityReport(gate=BoundaryCheck)
     ▼
Gate::Accepted → C++ inference → ajna_session_push_result()
```

### Zero-Copy Frame Path

- **iOS**: `CVPixelBuffer` locked `.readOnly` by `AjnaCameraAdapter` before `didReceiveFrame`. Unlocked immediately after `ajna_coreml_process()` returns. No copy.
- **Android**: `AHardwareBuffer` imported via `TfLiteInterpreterSetAHardwareBufferInput` (GPU delegate path). CPU path: locked with `AHardwareBuffer_lock`, copied to tensor, unlocked immediately.
- **WASM**: `ImageData → OwnedFrame` copy is **mandatory and documented**. The JS heap is not accessible to ONNX Runtime's C++ allocator. This is an expected cost at ~5 MB/s for 25fps/1080p.

## Performance Budget (Helio G85 Reference Device)

| Component | Budget | Implementation |
|-----------|--------|----------------|
| Quality gates (total) | < 4 ms | CPU-only, Laplacian + histogram + SAD + Sobel (4× subsampled) |
| TFLite GPU (Mali-G57) | < 45 ms | GPU delegate, AHardwareBuffer zero-copy |
| TFLite CPU XNNPACK | < 200 ms | Fallback path, 2 threads |
| Total pipeline heap | < 48 MB | Includes model weights (not in repo) |
| Frame budget at 25fps | 40 ms | 4ms gates + 30ms inference + 6ms margin |

## Post-Quantum Cryptography

### ML-DSA (FIPS 204 — CRYSTALS-Dilithium)

Ajna uses Dilithium-3 (ML-DSA Level 3) for result signing. Parameters:

| Parameter | Value |
|-----------|-------|
| Security level | 128-bit post-quantum |
| Public key | 1952 bytes |
| Secret key | 4000 bytes |
| Signature | 3293 bytes |
| NIST standard | FIPS 204 (August 2024) |

The signing target is `ScanResult::canonical_bytes()` — a deterministic, sorted, length-prefixed encoding of all document fields plus document type and issuing country.

### ML-KEM (FIPS 203 — CRYSTALS-Kyber)

Transport key encapsulation uses Kyber-1024:

| Parameter | Value |
|-----------|-------|
| Security level | 256-bit classical / 128-bit post-quantum |
| Public key | 1568 bytes |
| Ciphertext | 1568 bytes |
| Shared secret | 32 bytes → AES-256-GCM |
| NIST standard | FIPS 203 (August 2024) |

### Private Key Protection

- **Non-WASM targets**: `libc::mlock()` is called on the secret key allocation immediately after `generate()`. On `Drop`, the key is zeroed via volatile write and `munlock()`-ed.
- **WASM**: In-memory only. No OS-level memory protection available in the browser sandbox. This is a documented limitation — see SECURITY_MODEL.md.

## Camera Format Constraints

### Android
- Request: `ImageFormat.YUV_420_888`
- NV12 detection: `planes[1].pixelStride == 2` (NV12) vs `== 1` (YV12)
- Frame rate: locked at 25fps via `CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE(25, 25)`
- Buffer pool: 4 buffers (`ImageReader.newInstance(..., maxImages=4)`)

### iOS
- Request: `kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange`
- Full-range adjustment: if HAL returns full-range, exposure gate thresholds are widened
- Frame rate: locked at 25fps via `activeVideoMinFrameDuration = CMTimeMake(1, 25)`

## Entity Graph

Ajna delivers a PQC-signed `ScanResult` to the host application. The entity graph linking IDV results to FaceGuard face embeddings and Guardian device fingerprints is assembled **server-side by Ajna**. Ajna does not implement cross-product linking.
