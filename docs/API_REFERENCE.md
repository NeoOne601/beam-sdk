# Ajna SDK API Reference

This document covers all public types and functions in the Ajna SDK. Read alongside [INTEGRATION_GUIDE.md](INTEGRATION_GUIDE.md) and [SECURITY_MODEL.md](SECURITY_MODEL.md).

---

## Rust Core (`ajna-core`)

### `frame.rs`

#### `enum PixelFormat` (repr C)

| Variant | Value | Description |
|---------|-------|-------------|
| `Nv12` | 0 | YUV 4:2:0 semi-planar, UV interleaved. Native ISP output. |
| `Yuv420P` | 1 | YUV 4:2:0 planar, UV separate (older Android HALs). |
| `Rgba8` | 2 | 8-bit RGBA interleaved (WASM input). |

#### `struct RawFrame` (repr C)

Non-owning view of a camera frame. All pointer fields must remain valid while the struct is alive.

| Field | Type | Description |
|-------|------|-------------|
| `y_plane` | `*const u8` | Y (luma) plane. Valid for `height × y_stride` bytes. |
| `uv_plane` | `*const u8` | UV (chroma) plane. NV12: interleaved CbCr. |
| `width` | `u32` | Frame width in pixels. |
| `height` | `u32` | Frame height in pixels. |
| `y_stride` | `u32` | Y plane row stride (bytes; may exceed width). |
| `uv_stride` | `u32` | UV plane row stride (bytes). |
| `format` | `PixelFormat` | Pixel layout. |
| `timestamp_us` | `u64` | Capture timestamp (monotonic µs). |

**Safety**: Caller must hold the platform buffer lock (`CVPixelBufferLockBaseAddress` / `AHardwareBuffer_lock`) for the lifetime of any `RawFrame` pointing into that buffer.

#### `struct OwnedFrame`

Heap-allocated copy. Used on WASM. **Expected copy cost at 25fps/1080p: ~5 MB/s.**

```rust
fn from_rgba(rgba: &[u8], width: u32, height: u32, timestamp_us: u64) -> OwnedFrame
fn as_raw(&self) -> RawFrame
```

---

### `quality.rs`

#### `enum Gate` (ordered, repr usize)

`Received(0) → BlurCheck(1) → ExposureCheck(2) → MotionCheck(3) → BoundaryCheck(4) → Accepted(5)`

Short-circuit: rejection at gate N sets all N+1 scores to `0.0`.

#### `struct QualityReport`

| Field | Type | Description |
|-------|------|-------------|
| `gate_reached` | `Gate` | Highest gate completed before rejection (or `Accepted`). |
| `blur_score` | `f32` | Laplacian variance. `< blur_threshold` → rejected. |
| `mean_luma` | `f32` | Mean Y-plane luminance. `0.0` if rejected at BlurCheck. |
| `p95_luma` | `f32` | 95th-percentile Y-plane luminance. |
| `motion_score` | `f32` | Normalised inter-frame SAD. `0.0` if rejected before MotionCheck. |
| `edge_density` | `f32` | Sobel edge density. `0.0` if rejected before BoundaryCheck. |

#### `struct QualityGate`

| Field | Default | Description |
|-------|---------|-------------|
| `blur_threshold` | `80.0` | Minimum Laplacian variance. |
| `min_luma` | `40.0` | Minimum mean luminance. |
| `max_luma` | `220.0` | Maximum mean luminance. |
| `p95_luma_max` | `245.0` | Maximum P95 luminance (overexposure). |
| `motion_threshold` | `0.12` | Maximum inter-frame SAD. |
| `edge_min` | `0.08` | Minimum Sobel edge density. |

```rust
// Safety: frame.y_plane valid for width×height bytes
pub unsafe fn evaluate(&mut self, frame: &RawFrame) -> QualityReport
```

---

### `session.rs`

#### `enum SessionState` (repr C)

`Idle(0) → Scanning(1) → Inferring(2) → Complete(3) / Failed(4)`

#### `struct SessionConfig` (repr C)

| Field | Default | Description |
|-------|---------|-------------|
| `min_quality_frames` | `3` | Quality frames required before inference. |
| `timeout_ms` | `30000` | Session timeout (ms). |
| `adaptive_gate_limit` | `60` | Consecutive gate failures triggering relaxation. |
| `pqc_sign_result` | `true` | Enable ML-DSA signing. |
| `include_raw_mrz` | `false` | Include raw MRZ string in result. |

#### `struct ScanSession`

```rust
fn new(config: SessionConfig) -> ScanSession
fn start(&mut self, timestamp_us: u64)
fn record_quality_frame(&mut self) -> bool   // true → trigger inference
fn record_gate_fail(&mut self) -> bool        // true → apply adaptive relaxation
fn is_timed_out(&self, now_us: u64) -> bool
fn set_inferring(&mut self)
fn complete(&mut self, result: ScanResult)
fn fail(&mut self, reason: Option<&str>)
```

---

### `result.rs`

#### `struct DocumentField`

| Field | Type | Description |
|-------|------|-------------|
| `key` | `String` | Field identifier (e.g. `"surname"`, `"document_number"`). |
| `value` | `String` | Extracted value. |
| `confidence` | `f32` | Per-field model confidence, 0.0–1.0. |

#### `struct ScanResult`

| Field | Type | Description |
|-------|------|-------------|
| `fields` | `Vec<DocumentField>` | All extracted fields (unordered). |
| `raw_mrz` | `Option<String>` | Raw MRZ string (if `include_raw_mrz`). |
| `document_type` | `String` | ICAO type string (`"passport"`, `"id_card"`, …). |
| `issuing_country` | `String` | ISO 3166-1 alpha-3 country code. |
| `confidence` | `f32` | Overall scan confidence. |
| `pqc_signature` | `Vec<u8>` | ML-DSA Level-3 signature over `canonical_bytes()`. |
| `pqc_public_key` | `Vec<u8>` | Matching ML-DSA public key (1952 bytes). |

```rust
/// Deterministic byte encoding for signing. Sorts fields by key.
/// Identical results (any field order) produce identical bytes.
pub fn canonical_bytes(&self) -> Vec<u8>
```

---

### `crypto.rs`

#### `enum MlDsaLevel` (repr C)

| Variant | Classical Security | PQ Security | Signature Length | Public Key |
|---------|-------------------|-------------|-----------------|------------|
| `Level2` | 128-bit | 64-bit | 2420 B | 1312 B |
| `Level3` | 192-bit | 96-bit | 3293 B nominal (shipped PQClean Round-3 emits **3309 B** — size buffers for 3309) | 1952 B |
| `Level5` | 256-bit | 128-bit | 4595 B | 2592 B |

#### `struct PqcSigner`

```rust
pub fn generate(level: MlDsaLevel) -> Result<PqcSigner, CryptoError>
pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, CryptoError>
pub fn verify(
    level: MlDsaLevel,
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<bool, CryptoError>
pub fn public_key_bytes(&self) -> &[u8]
```

**Safety note (mlock)**: Private key bytes are `mlock()`-ed on non-WASM targets with the `mlock` feature (default). On `Drop`, bytes are zeroed via volatile write and `munlock()`-ed.

**Unsafe blocks**: The `mlock`/`munlock` calls use `unsafe { libc::mlock(...) }`. Justification: `mlock` is a POSIX syscall requiring a raw pointer; there is no safe abstraction in `libc`.

#### `struct MlKemSession`

```rust
pub fn encapsulate(server_public_key: &[u8]) -> Result<MlKemSession, CryptoError>
// ciphertext: 1568 bytes (Kyber-1024)
// shared_secret: 32 bytes → AES-256-GCM key material
```

---

### `pipeline.rs`

#### `enum PipelineResult`

```rust
Rejected(QualityReport)    // frame failed a gate
AcceptedForInference       // gate passed; session → Inferring
Complete(ScanResult)       // inference done; result available
```

#### `struct FramePipeline`

```rust
pub fn new(config: SessionConfig) -> FramePipeline
// Safety: frame.y_plane valid for frame.width×frame.height bytes
pub unsafe fn process_frame(&mut self, frame: &RawFrame, now_us: u64) -> PipelineResult
pub fn session_state(&self) -> SessionState
pub fn push_result(&mut self, result: ScanResult)
```

**Critical invariant**: `AcceptedForInference` is returned **only** when `gate_reached == Gate::Accepted`. The C++ GPU layer is never invoked on rejected frames.

---

### `ffi.rs` — C FFI Exports

All functions are `#[no_mangle] extern "C"` and safe to call from C/C++/ObjC.

> **Canonical signatures live in [`include/ajna_ffi.h`](../include/ajna_ffi.h)** —
> that header is what every platform bridge compiles against, and it wins on any
> difference. Post-VR-4, fallible functions return `int32_t` status codes
> (`0 = OK`, negative = error). `ajna_session_get_result_json` and
> `ajna_ui_config_validate` are also exported (see header).

```c
// Session lifecycle
AjnaSessionHandle ajna_session_create(AjnaSessionConfig config);
int32_t           ajna_session_destroy(AjnaSessionHandle handle);  // null-safe
int32_t           ajna_session_start(AjnaSessionHandle, uint64_t timestamp_us);
uint32_t          ajna_session_get_state(AjnaSessionHandle);       // returns SessionState

// Quality gate
AjnaGateHandle ajna_gate_create(void);
uint32_t       ajna_gate_evaluate(AjnaGateHandle, const AjnaRawFrame*); // returns Gate discriminant
int32_t        ajna_gate_destroy(AjnaGateHandle);  // null-safe

// Result ingestion (called by C++ inference layer).
// VR-1: nonce + session id are passed in and bound into the signed bytes.
int32_t ajna_session_push_result(
    AjnaSessionHandle, const CField*, size_t field_count,
    const uint8_t* doc_type, size_t doc_type_len,
    const uint8_t* country,  size_t country_len,
    const uint8_t* nonce_ptr, size_t nonce_len,
    const uint8_t* session_id_ptr, size_t session_id_len,
    float overall_conf, bool include_pqc_sig
);
```

**Safety documentation**: Every `unsafe extern "C"` function in `ffi.rs` documents its pointer preconditions. Null handles passed to `*_destroy` functions are silently ignored (null guard).

**Unsafe blocks**: `Box::from_raw(handle)` is used in destroy functions. Justification: the handle was created by `Box::into_raw()` in the matching create function; reclaiming it is the only correct way to free the allocation without a leak.

---

## Swift API (`AjnaSDK.swift`)

### `struct AjnaScanConfig`

```swift
var minQualityFrames: Int   // default 3
var timeoutMs:         Int  // default 30000
var pqcSignResult:     Bool // default true
```

### `struct AjnaScanResult : Codable`

```swift
let fields:         [AjnaDocumentField]
let rawMrz:         String?
let documentType:   String
let issuingCountry: String
let confidence:     Float
let pqcSignature:   Data
let pqcPublicKey:   Data
```

### `class AjnaScanner`

```swift
func configure() throws                     // throws AjnaScannerError
func startScan(
    config: AjnaScanConfig,
    completion: @escaping (Result<AjnaScanResult, AjnaScannerError>) -> Void
)
func stopScan()
```

Completion is always dispatched on `DispatchQueue.main`.

### `enum AjnaScannerError`

```swift
case noCameraAvailable
case modelNotFound(String)
case sessionTimeout
case inferenceFailure(String)
case configurationFailed(String)
```

---

## Kotlin API (`AjnaNativeBridge.kt`)

### `class AjnaNativeBridge`

```kotlin
fun nativeCreateInferenceEngine(modelPath: String): Long
fun nativeDestroyInferenceEngine(handle: Long)
fun nativeCreateSession(): Long
fun nativeDestroySession(handle: Long)
fun nativeStartSession(handle: Long, timestampUs: Long)
fun nativeOnFrame(
    engineHandle: Long, sessionHandle: Long,
    yBuffer: ByteBuffer, uvBuffer: ByteBuffer,
    width: Int, height: Int,
    yStride: Int, uvStride: Int,
    isNv12: Boolean, timestampUs: Long
)
fun nativeGetState(sessionHandle: Long): Int
// Convenience wrapper:
fun onFrame(...) // delegates to nativeOnFrame
fun getSessionState(sessionHandle: Long): SessionState
open fun onCameraError(errorCode: Int)
```

### `enum SessionState`

```kotlin
IDLE, SCANNING, INFERRING, COMPLETE, FAILED
```

Library loaded via `System.loadLibrary("ajna_sdk")` in companion object `init` block.
