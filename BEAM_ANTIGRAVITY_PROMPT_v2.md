# BEAM SDK — Google Antigravity 2.0 Manager View Project Brief
## Version: 2.1 (reviewed) | Model: Claude Opus 4.6 (preferred) or Gemini 3 Pro | Mode: Plan-Review-Execute

---

## INSTRUCTION TO ANTIGRAVITY MANAGER

This is a Plan-Review-Execute brief. Do NOT begin execution immediately.

1. Parse this brief into a structured Plan Artifact listing all 5 subagent workstreams,
   their dependencies, and their verification criteria.
2. Present the Plan Artifact for human review before any file is touched.
3. Execute only after explicit approval.
4. Use 5 parallel subagents in Manager view — one per workstream.
5. Each subagent must produce a named Artifact (file list + test output screenshot)
   before the orchestrator marks it complete.
6. CRITICAL: Do NOT regenerate or overwrite any file listed under "PROTECTED FILES" below.
   Read each protected file before writing any code that depends on it.

Preferred model per workstream:
- Workstreams 1, 2, 3 (systems/native code): Claude Opus 4.6
- Workstream 4 (build/CI): Gemini 3 Pro or Claude Sonnet 4.6
- Workstream 5 (documentation): Claude Sonnet 4.6

---

## PROJECT CONTEXT

Repository root: beam-sdk/
Mission: Complete and package the Beam SDK as production-grade, CI-validated,
         cross-platform native libraries for Surt AI's three products: IDV, Guardian, FaceGuard.
Architecture constraint: C++ at the ML runtime boundary (TFLite/CoreML/ONNX C++ APIs),
         Rust for all business logic. This split is non-negotiable — see constraints below.

---

## PROTECTED FILES — READ ONLY, DO NOT MODIFY OR REGENERATE

These files are already written and placed in the repository by the architect.
Every subagent must read these files before writing any dependent code.
Any subagent that overwrites a protected file will be restarted from scratch.

```
beam-sdk/
  core/src/quality.rs        -- Quality gates: blur, exposure, motion, boundary (223 lines, complete)
  core/src/crypto.rs         -- ML-DSA FIPS 204 + ML-KEM FIPS 203 (111 lines, STUBS — see Task 1.3)
  core/src/ffi.rs            -- C FFI boundary, all #[no_mangle] exports (151 lines, complete)
  platform/android/tflite_bridge.cpp    -- TFLite GPU/NNAPI/CPU C++ bridge (227 lines, complete)
  platform/android/BeamCameraAdapter.kt -- Camera2 Kotlin adapter (161 lines, complete)
  platform/ios/BeamCameraAdapter.swift  -- AVFoundation Swift adapter (132 lines, complete)
  build/CMakeLists.txt                  -- Cross-platform CMake (127 lines, complete)
  docs/BEAM_WHITEPAPER.md                    -- Architecture and PQC security whitepaper (350 lines)
```

---

## EMPTY SCAFFOLD FILES — TO BE WRITTEN BY SUBAGENTS

These files exist on disk as empty placeholders created by the folder scaffold command.
Subagents must write their full contents. Do not create new files at these paths — write
into the existing empty files.

```
-- Subagent 1 owns these:
beam-sdk/core/src/lib.rs               -- [Subagent 1 — Task 1.1]
beam-sdk/core/src/frame.rs             -- [Subagent 1 — Task 1.1]
beam-sdk/core/src/session.rs           -- [Subagent 1 — Task 1.1]
beam-sdk/core/src/result.rs            -- [Subagent 1 — Task 1.1]
beam-sdk/core/src/pipeline.rs          -- [Subagent 1 — Task 1.2]
beam-sdk/core/Cargo.toml               -- [Subagent 1 — Task 1.3]
beam-sdk/Cargo.toml                    -- [Subagent 1 — Task 1.3] workspace manifest

-- Subagent 1 also owns this protected file (stub completion only):
beam-sdk/core/src/crypto.rs            -- [Subagent 1 — Task 1.4] complete the stubs IN PLACE

-- Subagent 2 owns these:
beam-sdk/platform/ios/coreml_bridge.mm        -- [Subagent 2 — Task 2.1]
beam-sdk/platform/wasm/onnx_bridge.cpp        -- [Subagent 2 — Task 2.2]
beam-sdk/platform/android/BeamNativeBridge.kt -- [Subagent 2 — Task 2.3]
beam-sdk/platform/ios/BeamSDK.swift           -- [Subagent 2 — Task 2.4]

-- Subagent 3 owns these:
beam-sdk/tests/quality_gate_tests.rs          -- [Subagent 3 — Task 3.1]
beam-sdk/tests/session_state_tests.rs         -- [Subagent 3 — Task 3.2]
beam-sdk/tests/crypto_pqc_tests.rs            -- [Subagent 3 — Task 3.3]
beam-sdk/tests/ffi_integration_tests.cpp      -- [Subagent 3 — Task 3.4]

-- Subagent 4 owns these:
beam-sdk/.github/workflows/ci.yml             -- [Subagent 4 — Task 4.1]
beam-sdk/ci/android_build_matrix.yml          -- [Subagent 4 — Task 4.1]
beam-sdk/ci/ios_xcodebuild.sh                 -- [Subagent 4 — Task 4.2]
beam-sdk/ci/wasm_emscripten.sh                -- [Subagent 4 — Task 4.2]
beam-sdk/scripts/package_android_aar.sh       -- [Subagent 4 — Task 4.3]
beam-sdk/scripts/package_ios_xcframework.sh   -- [Subagent 4 — Task 4.3]
beam-sdk/scripts/package_wasm_npm.sh          -- [Subagent 4 — Task 4.3]

-- Subagent 5 owns these:
beam-sdk/docs/INTEGRATION_GUIDE.md            -- [Subagent 5 — Task 5.1]
beam-sdk/docs/SECURITY_MODEL.md               -- [Subagent 5 — Task 5.2]
beam-sdk/docs/API_REFERENCE.md                -- [Subagent 5 — Task 5.3]
beam-sdk/README.md                            -- [Subagent 5 — Task 5.4]
```

---

## ARCHITECTURAL CONSTRAINTS (non-negotiable — enforce in all generated code)

### Constraint 1: Language boundary

C++ owns: TFLite, CoreML, ONNX Runtime integration, AHardwareBuffer import,
  CVPixelBuffer to tensor transfer, all raw pointer arithmetic at the inference boundary.
Rust owns: frame type definitions, quality gate logic, session state machine,
  result parsing, PQC signing, FFI boundary declarations.
Swift/Kotlin own: camera session, format negotiation, JNI bridge, surface delivery.
Crossing the boundary: exactly one FFI call per accepted frame
  (beam_session_push_result in ffi.rs). Never call Rust from C++ more than once per frame.

### Constraint 2: Zero-copy path

iOS: CVPixelBuffer must be locked with CVPixelBufferLockBaseAddress(.readOnly) before
  any pointer is passed to C++. Unlock immediately after the pipeline tick returns.
Android: AHardwareBuffer must be imported via TfLiteInterpreterSetAHardwareBufferInput
  (GPU delegate) or locked with AHardwareBuffer_lock (CPU path). Never held across frames.
WASM: mandatory copy from ImageData to heap OwnedFrame before entering Rust.
  Document this clearly in all WASM-facing APIs as an expected cost.

### Constraint 3: Gate ordering is performance-critical

Quality gates MUST run in this exact order: Blur then Exposure then Motion then Boundary.
Read quality.rs before writing any code that touches the gate pipeline.
The C++/GPU layer must NEVER be invoked on a frame that did not reach Gate::Accepted.
Tests must assert this invariant explicitly.

### Constraint 4: Camera format negotiation

Android: request ImageFormat.YUV_420_888. Detect NV12 vs YV12 from
  planes[1].pixelStride (2 = NV12, 1 = YV12). Pass format flag to C++ layer.
iOS: request kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange explicitly.
  If HAL returns full-range, adjust exposure gate thresholds accordingly.
Frame rate: lock at 25fps on both platforms. Reason: 40ms per frame budget.
  Gates: 4ms. Inference: target under 30ms. Margin: 6ms for HAL jitter.

### Constraint 5: Budget device compliance (Helio G85 reference)

Quality gate total: under 4ms on Cortex-A55 at 2.0GHz, single thread.
ML inference TFLite CPU XNNPACK fallback: under 200ms.
ML inference TFLite GPU delegate Mali-G57: under 45ms.
Memory: total pipeline heap allocation under 48MB.
Subagent 3 must write criterion benchmark tests asserting these budgets.

### Constraint 6: Post-quantum cryptography (ML-DSA FIPS 204)

Read crypto.rs before completing its stubs. The PqcSigner struct and MlKemSession
struct are already defined. Subagent 1 completes only the stub function bodies using
the pqcrypto-dilithium crate. Do not change the public API surface of crypto.rs.
The ML-DSA Level 3 (Dilithium-3) implementation must:
- Accept a 32-byte seed for deterministic keypair generation (for test reproducibility)
- Sign the canonical_bytes() output of ScanResult
- Use mlock() on private key bytes on non-WASM targets via libc::mlock
- Test: sign to verify round-trip must pass. Signature length must equal 3293 bytes.

---

## SUBAGENT 1 — Rust Core Completion

Owner: Claude Opus 4.6
Dependency: none (runs in parallel from start)
First action: Read quality.rs, ffi.rs, and crypto.rs before writing a single line.

### Task 1.1 — Write the four core Rust modules into their empty scaffold files

Write beam-sdk/core/src/lib.rs:
- Re-export the public API surface: RawFrame, OwnedFrame, FramePipeline, PipelineResult,
  QualityGate, QualityReport, Gate, ScanSession, SessionConfig, SessionState,
  ScanResult, DocumentField, PqcSigner, MlDsaLevel, MlKemSession
- No logic in this file — facade re-exports only
- cfg_attr no_std with extern crate alloc for cross-platform compatibility

Write beam-sdk/core/src/frame.rs:
- RawFrame: non-owning C-compatible struct with y_plane, uv_plane, width, height,
  y_stride, uv_stride, PixelFormat enum, timestamp_us
- OwnedFrame: heap-allocated WASM copy with as_raw() method
- PixelFormat enum: Nv12, Yuv420P, Rgba8
- All repr(C) for FFI compatibility
- Safety documentation on every raw pointer field

Write beam-sdk/core/src/session.rs:
- SessionState enum: Idle, Scanning, Inferring, Complete, Failed (repr(C))
- SessionConfig struct with defaults: min_quality_frames 3, timeout_ms 30000,
  adaptive_gate_limit 60, pqc_sign_result true, include_raw_mrz false
- ScanSession struct owning config, state, quality_frame_count, consecutive_gate_fails,
  start_timestamp_us, result Option<ScanResult>
- Methods: new(), start(), record_quality_frame() -> bool, record_gate_fail() -> bool,
  is_timed_out(), set_inferring(), complete(), fail()

Write beam-sdk/core/src/result.rs:
- DocumentField struct: key String, value String, confidence f32
- ScanResult struct: fields Vec<DocumentField>, raw_mrz Option<String>,
  document_type String, issuing_country String, confidence f32,
  pqc_signature Vec<u8>, pqc_public_key Vec<u8>
- canonical_bytes() method: length-prefixed UTF-8 key=value pairs sorted by key,
  NUL delimited, deterministic

### Task 1.2 — Write pipeline.rs into its empty scaffold file

Write beam-sdk/core/src/pipeline.rs:
- PipelineResult enum: Rejected(QualityReport), AcceptedForInference, Complete(ScanResult)
- FramePipeline struct owning a QualityGate and ScanSession
- process_frame(frame: &RawFrame, now_us: u64) -> PipelineResult
  -- Check session timeout first, return Rejected if timed out
  -- Call quality_gate.evaluate(frame)
  -- If gate_reached != Gate::Accepted, call session.record_gate_fail()
     and trigger adaptive relaxation if that returns true
  -- If Gate::Accepted, call session.record_quality_frame()
     and return AcceptedForInference if that returns true
  -- If session state is Complete, return Complete(result)
- Adaptive relaxation: when triggered, reduce blur_threshold by 15%,
  widen exposure range by 15%, increase motion_threshold by 15%
- new(config: SessionConfig) -> Self

### Task 1.3 — Write Cargo.toml files

Write beam-sdk/core/Cargo.toml:
```toml
[package]
name = "beam-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib", "cdylib"]

[dependencies]
pqcrypto-dilithium = "0.5"
pqcrypto-kyber = "0.8"
pqcrypto-traits = "0.3"
libc = { version = "0.2", optional = true }
getrandom = { version = "0.2", features = ["js"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"

[features]
default = ["mlock"]
mlock = ["libc"]

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"

[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
```

Write beam-sdk/Cargo.toml (workspace manifest):
```toml
[workspace]
members = ["core"]
resolver = "2"
```

### Task 1.4 — Complete the crypto.rs stubs IN PLACE

Read the existing crypto.rs. Do not change any struct definitions, enum definitions,
or public method signatures. Replace only the stub function bodies:

PqcSigner::generate(level) — replace stub body with:
  use pqcrypto_dilithium::dilithium3::{keypair, PublicKey, SecretKey};
  let (pk, sk) = keypair();
  Use sk.as_bytes() and pk.as_bytes() for the stored Vec<u8> values.
  On non-WASM targets with feature "mlock", call libc::mlock on the sk bytes.

PqcSigner::sign(message) — replace stub body with:
  use pqcrypto_dilithium::dilithium3::sign;
  use pqcrypto_traits::sign::SignedMessage;
  Return the signature bytes extracted from the signed message.

PqcSigner::verify(level, public_key, message, signature) — replace stub body with:
  use pqcrypto_dilithium::dilithium3::{open, PublicKey};
  Reconstruct PublicKey from bytes, call open(), return Ok(true) or Ok(false).

MlKemSession::encapsulate(server_public_key) — replace stub body with:
  use pqcrypto_kyber::kyber1024::{encapsulate, PublicKey};
  Return the real ciphertext and shared_secret bytes.

Verification Artifact: cargo build --release --target aarch64-linux-android exits 0.
Screenshot the terminal output.

---

## SUBAGENT 2 — Platform Bridges Completion

Owner: Claude Opus 4.6
Dependency: none (runs in parallel from start)
First action: Read ffi.rs to understand the C API surface before writing any bridge code.

### Task 2.1 — coreml_bridge.mm

Write beam-sdk/platform/ios/coreml_bridge.mm:
- Load a .mlpackage model from the app bundle using [MLModel modelWithContentsOfURL:]
- Accept a CVPixelBufferRef already locked by the Swift adapter
- Create MLFeatureValue from the CVPixelBuffer — this is the zero-copy path
- Run [model predictionFromFeatures:options:error:] with
  MLPredictionOptions.usesCPUOnly = NO (ANE dispatch enabled)
- On ANE unavailable (check MLComputeUnits), fall back to CPU
- Call beam_session_push_result() from ffi.rs with decoded output
- Expose plain C API:
    beam_coreml_session_t* beam_coreml_create(const char* model_path)
    void beam_coreml_process(beam_coreml_session_t*, CVPixelBufferRef, BeamSessionHandle)
    void beam_coreml_destroy(beam_coreml_session_t*)

### Task 2.2 — onnx_bridge.cpp

Write beam-sdk/platform/wasm/onnx_bridge.cpp:
- Include onnxruntime_cxx_api.h
- Accept a raw RGBA uint8 pointer + dimensions (mandatory copy from WASM adapter)
- Convert RGBA to normalised float tensor (unavoidable on WASM — document this)
- Run inference via Ort::Session::Run()
- Use WebGPU execution provider when available:
    OrtSessionOptionsAppendExecutionProvider_WebGPU()
- Fall back to WASM SIMD CPU path
- Expose:
    extern "C" void* beam_wasm_create(const char* model_path)
    extern "C" void beam_wasm_process_frame(void* session, uint8_t* rgba,
                                             int w, int h, void* rust_session_handle)
    extern "C" void beam_wasm_destroy(void* session)

### Task 2.3 — BeamNativeBridge.kt

Write beam-sdk/platform/android/BeamNativeBridge.kt:
- Package: ai.surt.beam
- Declare all external fun JNI methods matching tflite_bridge.cpp exports:
    external fun nativeCreateInferenceEngine(modelPath: String): Long
    external fun nativeDestroyInferenceEngine(handle: Long)
    external fun nativeCreateSession(): Long
    external fun nativeDestroySession(handle: Long)
    external fun nativeStartSession(handle: Long, timestampUs: Long)
    external fun nativeOnFrame(engineHandle: Long, sessionHandle: Long,
                                yBuffer: ByteBuffer, uvBuffer: ByteBuffer,
                                width: Int, height: Int,
                                yStride: Int, uvStride: Int,
                                isNv12: Boolean, timestampUs: Long)
    external fun nativeGetState(sessionHandle: Long): Int
- Companion object with System.loadLibrary("beam_sdk")
- Kotlin-friendly wrapper: fun onFrame(...) calling BeamCameraAdapter output directly

### Task 2.4 — BeamSDK.swift

Write beam-sdk/platform/ios/BeamSDK.swift:
- Public class BeamScanner: NSObject implementing BeamFrameDelegate
- Owns a BeamCameraAdapter and calls beam_coreml_process via @_silgen_name bridge
- Public API:
    func configure() throws
    func startScan(config: BeamScanConfig, completion: @escaping (BeamScanResult) -> Void)
    func stopScan()
- BeamScanConfig: minQualityFrames, timeoutMs, pqcSignResult (mirrors SessionConfig)
- BeamScanResult: Codable struct mirroring ScanResult from Rust
- All completion callbacks dispatched on DispatchQueue.main
- BeamError enum: noCameraAvailable, modelNotFound, sessionTimeout, inferenceFailure

Verification Artifact: xcodebuild -scheme BeamSDK -sdk iphonesimulator build exits 0.

---

## SUBAGENT 3 — Test Suite

Owner: Claude Opus 4.6
Dependency: Subagent 1 must complete all of Tasks 1.1, 1.2, 1.3, and 1.4 before
            any test can compile. Do not begin until Subagent 1 Verification Artifact
            is confirmed.
First action: Read quality.rs, session.rs, result.rs, crypto.rs, ffi.rs, and pipeline.rs
              before writing a single test.

### Task 3.1 — quality_gate_tests.rs

Write beam-sdk/tests/quality_gate_tests.rs:
- Test: sharp 64x64 Y-plane crop → blur_score > 80.0 (passes gate)
- Test: blurred crop filled with constant value → blur_score < 5.0 (fails gate)
- Test: dark frame mean luma 20 → fails exposure gate, gate_reached == Gate::ExposureCheck
- Test: overexposed frame P95 luma 248 → fails exposure gate
- Test: identical consecutive crops → motion_score < 0.01
- Test: fully random crop vs previous → motion_score > 0.12 (fails gate)
- Test: flat grey frame → edge_density < 0.02 (fails boundary gate)
- Test: frame containing a white rectangle on black → edge_density > 0.08 (passes)
- CRITICAL INVARIANT TEST: frame that fails Gate::BlurCheck must have exposure_score,
  motion_score, and edge_density all equal to 0.0 — verify short-circuit is enforced

### Task 3.2 — session_state_tests.rs

Write beam-sdk/tests/session_state_tests.rs:
- Test: ScanSession::new() → state == SessionState::Idle
- Test: session.start(0) → state == SessionState::Scanning
- Test: record_quality_frame() called 3 times → third call returns true
- Test: record_gate_fail() called 60 times → 60th call returns true (relaxation triggered)
- Test: is_timed_out() with now_us < start + 30_000_000 → false
- Test: is_timed_out() with now_us >= start + 30_000_000 → true
- Test: complete(result) → state == SessionState::Complete, session.result is Some
- Test: fail() → state == SessionState::Failed
- Test: calling record_quality_frame() on a Complete session does not change state

### Task 3.3 — crypto_pqc_tests.rs

Write beam-sdk/tests/crypto_pqc_tests.rs:
- Test: PqcSigner::generate(MlDsaLevel::Level3) → public_key_bytes().len() == 1952
- Test: sign() output length == 3293
- Test: sign then verify round-trip → Ok(true)
- Test: sign message "hello", verify against message "hello!" → Ok(false) or Err
- Test: ScanResult::canonical_bytes() with same fields called twice → identical bytes
- Test: two ScanResults with same fields in different insertion order →
        canonical_bytes() identical (sort is enforced)
- Test: MlKemSession::encapsulate() → ciphertext.len() == 1568
- SECURITY TEST: assert that the private key bytes do not appear as a
                 contiguous subsequence anywhere in the signature output

### Task 3.4 — ffi_integration_tests.cpp

Write beam-sdk/tests/ffi_integration_tests.cpp:
- Hand-write or cbindgen-generate beam_ffi.h header mirroring all #[no_mangle]
  exports in ffi.rs: beam_session_create, beam_session_destroy, beam_session_start,
  beam_gate_create, beam_gate_evaluate, beam_gate_destroy, beam_session_push_result
- Test: beam_session_create(default_config) returns non-null handle
- Test: beam_gate_create() returns non-null handle
- Test: beam_session_start(handle, 0) — no crash, state is Scanning (readable via ABI)
- Test: beam_session_push_result() with valid CField array → getState returns Complete (4)
- Test: beam_session_destroy() on valid handle → no crash
- Test: beam_session_destroy(nullptr) → no crash (null guard in ffi.rs)
- MEMORY TEST: run with valgrind --leak-check=full. Must show 0 bytes definitely lost.
- Include a criterion benchmark asserting gate evaluate loop completes in under 4ms
  on 1920x1080 synthetic Y-plane data

Verification Artifact: cargo test --release 2>&1 | grep -E "test result|FAILED"
Screenshot showing all tests pass, 0 failed.

---

## SUBAGENT 4 — Build System and Packaging

Owner: Gemini 3 Pro or Claude Sonnet 4.6
Dependency: Subagents 1 and 2 must have their Verification Artifacts confirmed before
            the packaging scripts can be written. The CI workflow can be written in parallel.

### Task 4.1 — GitHub Actions CI workflow

NOTE: GitHub Actions requires workflows at .github/workflows/ not ci/.
Write beam-sdk/.github/workflows/ci.yml (create the .github/workflows/ directories if needed):

```yaml
name: Beam SDK CI
on: [push, pull_request]

jobs:
  android-arm64:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: android-actions/setup-android@v3
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android
      - name: Install clang (required by pqcrypto-dilithium)
        run: sudo apt-get install -y clang libclang-dev
      - name: Install Android NDK r26
        run: sdkmanager "ndk;26.1.10909125"
      - name: Build Rust core (aarch64-android)
        run: cargo build --release --target aarch64-linux-android
        working-directory: core
      - name: CMake configure and build
        run: |
          cmake -DCMAKE_TOOLCHAIN_FILE=$ANDROID_NDK/build/cmake/android.toolchain.cmake \
                -DANDROID_ABI=arm64-v8a \
                -DANDROID_PLATFORM=android-24 \
                -DBEAM_TARGET=Android \
                ../build
          cmake --build . --config Release
        working-directory: build_output
      - name: Package AAR
        run: bash scripts/package_android_aar.sh

  ios-arm64:
    runs-on: macos-15
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-ios
      - name: Build Rust core (iOS)
        run: cargo build --release --target aarch64-apple-ios
        working-directory: core
      - name: Build iOS XCFramework
        run: bash ci/ios_xcodebuild.sh
      - name: Package XCFramework
        run: bash scripts/package_ios_xcframework.sh

  wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: mymindstorm/setup-emsdk@v14
        with:
          version: "3.1.50"
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - name: Install wasm-pack
        run: cargo install wasm-pack
      - name: Build WASM
        run: bash ci/wasm_emscripten.sh

  rust-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install clang (required by pqcrypto-dilithium)
        run: sudo apt-get install -y clang libclang-dev
      - name: Run all Rust tests
        run: cargo test --release
        working-directory: core
      - name: Run benchmarks (verify budget)
        run: cargo bench
        working-directory: core
```

Also write beam-sdk/ci/android_build_matrix.yml — a separate matrix job that builds
for arm64-v8a, armeabi-v7a, and x86_64 ABIs in parallel, for use in release pipelines.

### Task 4.2 — Shell scripts for CI helpers

Write beam-sdk/ci/ios_xcodebuild.sh:
- xcodebuild -scheme BeamSDK -sdk iphoneos -arch arm64 build
  followed by xcodebuild -scheme BeamSDK -sdk iphonesimulator -arch x86_64 build
- Exit non-zero on any failure

Write beam-sdk/ci/wasm_emscripten.sh:
- rustup target add wasm32-unknown-unknown
- emcmake cmake with BEAM_TARGET=WASM pointing at build/CMakeLists.txt
- emmake make
- Assert output .wasm file exists, exit non-zero if not

### Task 4.3 — Packaging scripts

Write beam-sdk/scripts/package_android_aar.sh:
- Build libbeam_sdk.so for arm64-v8a and armeabi-v7a via cmake
- Bundle both .so files + libbeam_core.a into standard AAR directory structure:
    jni/arm64-v8a/libbeam_sdk.so
    jni/armeabi-v7a/libbeam_sdk.so
    classes.jar (compiled BeamNativeBridge.kt via kotlinc)
- Zip into dist/BeamSDK-0.1.0-release.aar
- Sign with debug keystore if KEYSTORE_PATH environment variable is not set
- Print the output path on success

Write beam-sdk/scripts/package_ios_xcframework.sh:
- Invoke ci/ios_xcodebuild.sh to produce device and simulator archives
- xcodebuild -create-xcframework merging both slices
- Include BeamSDK.swift compiled .swiftmodule
- Output: dist/BeamSDK.xcframework/

Write beam-sdk/scripts/package_wasm_npm.sh:
- Run ci/wasm_emscripten.sh
- Copy beam_sdk.wasm and beam_sdk.js into dist/npm/
- Write dist/npm/package.json with name @surt/beam-sdk, version 0.1.0, main beam_sdk.js
- Write dist/npm/beam-sdk.d.ts TypeScript declaration file with:
    BeamModule, BeamSession, BeamScanResult, BeamScanConfig types
- npm pack dist/npm/ to produce dist/BeamSDK-0.1.0.tgz

Verification Artifact: run all three packaging scripts and screenshot the dist/ directory
listing showing BeamSDK-0.1.0-release.aar, BeamSDK.xcframework/, and BeamSDK-0.1.0.tgz.

---

## SUBAGENT 5 — Documentation

Owner: Claude Sonnet 4.6
Dependency: Subagents 1 through 4 must complete so all API surfaces are finalised.
First action: Read WHITEPAPER.md, ffi.rs, BeamSDK.swift, and BeamNativeBridge.kt
              before writing any documentation.

### Task 5.1 — INTEGRATION_GUIDE.md

Write beam-sdk/docs/INTEGRATION_GUIDE.md covering three platform sections:

Android section:
- Add implementation fileTree(include: ['*.aar']) to app/build.gradle
- Initialise BeamNativeBridge in Application.onCreate
- Wire BeamCameraAdapter.delegate to BeamNativeBridge.onFrame
- Handle ScanResultParcel in a callback
- Complete minimal code sample under 20 lines that compiles

iOS section:
- Add BeamSDK.xcframework to Xcode project under Frameworks
- Instantiate BeamScanner, call configure() then startScan()
- Handle BeamScanResult in the completion closure
- Complete minimal code sample under 20 lines that compiles

Web/WASM section:
- npm install @surt/beam-sdk
- Import BeamModule, call BeamModule.beam_wasm_create() with model path
- Note the mandatory ImageData copy boundary explicitly — this is not a bug
- Complete minimal TypeScript sample under 20 lines

### Task 5.2 — SECURITY_MODEL.md

Write beam-sdk/docs/SECURITY_MODEL.md covering:
- Why ML-DSA FIPS 204 replaces ECDSA for result signing (harvest-now-decrypt-later threat)
- NIST standardisation timeline: FIPS 203, 204, 205 published August 2024
- Key storage per platform: Secure Enclave (iOS), StrongBox Keymaster (Android),
  in-memory only on WASM (document this limitation explicitly)
- Hybrid classical plus PQC transition recommendation for 2025 to 2028
- Transport security: ML-KEM-1024 session key, TLS 1.3 minimum, certificate pinning
- Attack surface inventory: adversarial documents, camera feed injection,
  gralloc buffer pool starvation, JNI boundary misuse, mlock bypass

### Task 5.3 — API_REFERENCE.md

Write beam-sdk/docs/API_REFERENCE.md:
- Read every file listed under PROTECTED FILES and every file written by Subagents 1 and 2
- Document all pub structs, enums, and functions in lib.rs, session.rs, result.rs, crypto.rs
- Document all #[no_mangle] functions in ffi.rs with full parameter and lifetime documentation
- Document the public Swift API in BeamSDK.swift
- Document the public Kotlin API in BeamNativeBridge.kt
- For every unsafe block in the codebase, include a one-sentence justification

### Task 5.4 — README.md

Write beam-sdk/README.md:
- One paragraph: what Beam is, which three Surt products use it, why it exists
- Platform support matrix table: iOS arm64, Android arm64-v8a armeabi-v7a, WASM
- Quick start: three commands to build for each platform
- Security note: ML-DSA PQC signing is enabled by default;
  to disable in dev set SessionConfig.pqc_sign_result = false
- Link to docs/INTEGRATION_GUIDE.md, docs/SECURITY_MODEL.md, docs/API_REFERENCE.md
- Do not invent a license — leave a TODO placeholder

Verification Artifact: markdownlint docs/*.md README.md exits 0 (no lint errors).

---

## SUCCESS CRITERIA (all must pass before session is marked complete)

| Criterion                                           | How verified                     |
|-----------------------------------------------------|----------------------------------|
| Android AAR builds for arm64-v8a                    | CI android-arm64 job green       |
| iOS XCFramework builds for arm64                    | CI ios-arm64 job green           |
| WASM module builds via Emscripten                   | CI wasm job green                |
| All Rust unit tests pass                            | cargo test 0 failures            |
| ML-DSA sign to verify round-trip passes             | crypto_pqc_tests.rs              |
| Gate invariant: no GPU on rejected frames           | ffi_integration_tests.cpp        |
| Gate total under 4ms on A55 benchmark               | criterion benchmark output       |
| Memory: no leaks in FFI create/push/destroy cycle   | valgrind 0 bytes definitely lost |
| All three dist packages exist                       | package scripts screenshot       |
| Markdown docs lint clean                            | markdownlint exit 0              |
| No protected file was modified                      | git diff on protected file list  |

---

## NOTES FOR ORCHESTRATOR

The pqcrypto-dilithium crate requires libclang at compile time. Every CI job that builds
the Rust core on Ubuntu must run: sudo apt-get install -y clang libclang-dev before cargo build.
This is already included in the ci.yml above — do not remove it.

WASM target setup requires two commands before the build:
  rustup target add wasm32-unknown-unknown
  cargo install wasm-pack
These are included in the CI workflow above.

If coreml_bridge.mm fails to compile on the macOS runner due to Xcode version mismatch,
fall back to a stub implementation that returns a ScanResult with confidence 0.0 and
logs "CoreML bridge: stub mode — no real model loaded" to stderr.
The pipeline must remain architecturally complete even without a live ML model.

Do not introduce any ML model weights into the repository. All C++/ObjC/WASM inference
bridges accept a model path as a runtime parameter. This is intentional.

The entity graph linking IDV results to FaceGuard face embeddings to Guardian device
fingerprints is assembled server-side by Surt. Beam's responsibility ends at delivering
a PQC-signed ScanResult to the host application. Do not implement cross-product linking.

After all subagents complete, run: git diff --name-only on the protected files list.
If any protected file appears in the diff, the session has failed. Revert and investigate.
