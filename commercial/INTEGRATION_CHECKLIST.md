# Beam Verify — Integration Checklist

## Overview

This checklist guides integrators through the four phases of Beam Verify SDK integration. Each phase has clear deliverables and acceptance criteria.

---

## Phase 1: Environment Setup (Day 1–2)

### Android
- [ ] Add Maven repository to `build.gradle`:
  ```groovy
  repositories { maven { url 'https://maven.pkg.github.com/surt-ai/beam-sdk' } }
  ```
- [ ] Add dependency: `implementation 'ai.surt.beam:beam-verify:0.1.0'`
- [ ] Add camera permission to `AndroidManifest.xml`
- [ ] Set `minSdk = 26` and `targetSdk = 34`
- [ ] Verify build succeeds on arm64-v8a

### iOS
- [ ] Add SPM dependency:
  ```swift
  .package(url: "https://github.com/surt-ai/beam-sdk", from: "0.1.0")
  ```
- [ ] Set deployment target to iOS 14.0
- [ ] Add `NSCameraUsageDescription` to `Info.plist`
- [ ] Verify build succeeds on arm64

### Web / WASM
- [ ] Install: `npm install @surt-ai/beam-verify`
- [ ] Import and initialise in your entry point
- [ ] Verify WASM loads in target browser (Chrome 120+, Safari 17+)

### Backend
- [ ] Deploy backend service (Docker Compose or Kubernetes)
- [ ] Run database migrations: `psql -f 001_initial.sql`
- [ ] Configure `DATABASE_URL` and `REDIS_URL` environment variables
- [ ] Verify `/health` endpoint returns `{"status":"ok"}`

**Phase 1 Acceptance**: SDK builds, backend health check passes.

---

## Phase 2: Camera Integration (Day 3–5)

### Android
- [ ] Initialise `BeamCameraAdapter` with Camera2 API
- [ ] Configure `ImageReader` with `YUV_420_888` format, 4 buffers
- [ ] Lock frame rate at 25fps via `CONTROL_AE_TARGET_FPS_RANGE`
- [ ] Verify NV12 detection via `planes[1].pixelStride`

### iOS
- [ ] Initialise `BeamCameraAdapter` with `AVCaptureSession`
- [ ] Request `kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange`
- [ ] Lock frame rate at 25fps via `activeVideoMinFrameDuration`
- [ ] Verify `CVPixelBufferLockBaseAddress` / unlock lifecycle

### Web
- [ ] Request camera via `getUserMedia` with environment-facing preference
- [ ] Create Web Worker for frame processing
- [ ] Implement `ImageData → OwnedFrame` copy path

**Phase 2 Acceptance**: Camera frames flowing through quality gates, gate status visible in UI.

---

## Phase 3: Inference + PQC (Day 6–8)

- [ ] Place model file in correct location per platform:
  - Android: `assets/beam_idv_v1.tflite`
  - iOS: Include `beam_idv_v1.mlpackage` in Xcode target
  - Web: Serve `beam_idv_v1.onnx` from CDN
- [ ] Verify model SHA256 matches `model_manifest.json`
- [ ] Confirm inference output follows `output_schema.json`
- [ ] Verify PQC signature is present in `ScanResult.pqc_signature`
- [ ] Validate signature size: 3,309 bytes (pqcrypto-dilithium 0.5)

**Phase 3 Acceptance**: `ScanResult` contains extracted fields with valid PQC signature.

---

## Phase 4: Backend Verification + Go-Live (Day 9–10)

- [ ] Implement nonce flow: `POST /v1/nonce` → `POST /v1/verify`
- [ ] Handle verification response in UI (success / failure states)
- [ ] Configure webhook URL for event delivery (optional)
- [ ] Set up audit log monitoring
- [ ] Perform end-to-end test with real document
- [ ] Review performance on target device (see `compliance/DEVICE_BENCHMARK_GUIDE.md`)
- [ ] Review security model (see `docs/SECURITY_MODEL.md`)
- [ ] File EAR self-classification if exporting (see `compliance/EXPORT_CONTROL.md`)

**Phase 4 Acceptance**: End-to-end scan → verify flow works with real document on target device.

---

## Post-Integration

- [ ] Set up monitoring for `/health` endpoint
- [ ] Configure alerting on verification failure rate > 5%
- [ ] Schedule device benchmark review (quarterly)
- [ ] Subscribe to Surt AI security advisories
- [ ] Plan Phase 2 roadmap items (Secure Enclave / StrongBox, NFC)
