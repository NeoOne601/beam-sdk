# BEAM SDK — Google Antigravity 2.0 Manager View Project Brief
## Version: 2.0 | Model: Claude Opus 4.6 (preferred) or Gemini 3 Pro | Mode: Plan-Review-Execute

---

## INSTRUCTION TO ANTIGRAVITY MANAGER

This is a **Plan-Review-Execute** brief. Do NOT begin execution immediately.

1. Parse this brief into a structured Plan Artifact listing all 5 subagent workstreams,
   their dependencies, and their verification criteria.
2. Present the Plan Artifact for human review before any file is touched.
3. Execute only after explicit approval.
4. Use **5 parallel subagents** in Manager view — one per workstream.
5. Each subagent must produce a **named Artifact** (file list + test output screenshot)
   before the orchestrator marks it complete.

Preferred model per workstream:
- Workstreams 1, 2, 3 (systems/native code): Claude Opus 4.6
- Workstream 4 (build/CI): Gemini 3 Pro or Claude Sonnet 4.6
- Workstream 5 (documentation + whitepaper): Claude Sonnet 4.6

---

## PROJECT CONTEXT

**Repository**: `beam-sdk/` (already scaffolded — see existing files below)
**Mission**: Package the full Beam SDK as production-grade, CI-validated, cross-platform
             native libraries for Surt AI's three products: IDV, Guardian, FaceGuard.
**Architecture constraint**: C++ at ML runtime boundary (TFLite/CoreML/ONNX C++ APIs),
             Rust for all business logic. This split is non-negotiable — see JD rationale below.

### Existing files (do not overwrite, extend or complete stubs):
```
beam-sdk/
  core/src/lib.rs          — Rust crate root (complete)
  core/src/frame.rs        — RawFrame + OwnedFrame (complete)
  core/src/quality.rs      — Quality gates: blur, exposure, motion, boundary (complete)
  core/src/session.rs      — ScanSession state machine (complete)
  core/src/result.rs       — ScanResult + DocumentField (complete)
  core/src/crypto.rs       — ML-DSA FIPS 204 + ML-KEM FIPS 203 stubs (STUBS — complete)
  core/src/ffi.rs          — C FFI boundary (complete)
  platform/android/tflite_bridge.cpp     — TFLite GPU/NNAPI/CPU C++ bridge (complete)
  platform/android/BeamCameraAdapter.kt  — Camera2 Kotlin adapter (complete)
  platform/ios/BeamCameraAdapter.swift   — AVFoundation Swift adapter (complete)
  build/CMakeLists.txt                   — Cross-platform CMake (complete)
```

### Files to be CREATED by this session (subagent assignments below):
```
core/Cargo.toml                          — [Subagent 1]
core/src/pipeline.rs                     — [Subagent 1]
platform/ios/coreml_bridge.mm            — [Subagent 2]
platform/wasm/onnx_bridge.cpp            — [Subagent 2]
platform/android/BeamNativeBridge.kt     — [Subagent 2]
platform/ios/BeamSDK.swift               — [Subagent 2]
tests/quality_gate_tests.rs              — [Subagent 3]
tests/session_state_tests.rs             — [Subagent 3]
tests/ffi_integration_tests.cpp          — [Subagent 3]
tests/crypto_pqc_tests.rs                — [Subagent 3]
ci/github_actions_workflow.yml           — [Subagent 4]
ci/android_build_matrix.yml             — [Subagent 4]
ci/ios_xcodebuild.sh                     — [Subagent 4]
ci/wasm_emscripten.sh                    — [Subagent 4]
scripts/package_android_aar.sh           — [Subagent 4]
scripts/package_ios_xcframework.sh       — [Subagent 4]
scripts/package_wasm_npm.sh              — [Subagent 4]
docs/INTEGRATION_GUIDE.md               — [Subagent 5]
docs/SECURITY_MODEL.md                  — [Subagent 5]
docs/API_REFERENCE.md                   — [Subagent 5]
README.md                               — [Subagent 5]
```

---

## ARCHITECTURAL CONSTRAINTS (non-negotiable — enforce in all generated code)

### Constraint 1: Language boundary
- **C++** owns: TFLite, CoreML, ONNX Runtime integration, AHardwareBuffer import,
  CVPixelBuffer → tensor transfer, all raw pointer arithmetic at the inference boundary.
- **Rust** owns: frame type definitions, quality gate logic, session state machine,
  result parsing, PQC signing, FFI boundary declarations.
- **Swift/Kotlin** own: camera session, format negotiation, JNI bridge, surface delivery.
- **Crossing the boundary**: exactly one FFI call per accepted frame
  (`beam_session_push_result` in ffi.rs). Never call Rust from C++ more than once per frame.

### Constraint 2: Zero-copy path
- iOS: CVPixelBuffer must be locked with `CVPixelBufferLockBaseAddress(.readOnly)` before
  any pointer is passed to C++. Unlocked immediately after the pipeline tick returns.
- Android: AHardwareBuffer must be imported via `TfLiteInterpreterSetAHardwareBufferInput`
  (GPU delegate) or locked with `AHardwareBuffer_lock` (CPU path). Never held across frames.
- WASM: mandatory copy from ImageData → heap OwnedFrame before entering Rust.
  Document this clearly in all WASM-facing APIs as an expected cost.

### Constraint 3: Gate ordering is performance-critical
Quality gates MUST run in this exact order: Blur → Exposure → Motion → Boundary.
Blur and Motion use only the 64×64 centre crop (Y-plane). Exposure and Boundary use the
full Y-plane (subsampled 4×). The C++/GPU layer must NEVER be invoked on a frame that
did not reach Gate::Accepted. Tests must assert this invariant.

### Constraint 4: Camera format negotiation
- Android: request `ImageFormat.YUV_420_888`. Detect NV12 vs YV12 from
  `planes[1].pixelStride` (2 = NV12, 1 = YV12). Pass format flag to C++ layer.
- iOS: request `kCVPixelFormatType_420YpCbCr8BiPlanarVideoRange` explicitly.
  If HAL returns full-range, adjust exposure gate thresholds accordingly.
- Frame rate: lock at 25fps on both platforms. Reason: 40ms/frame budget.
  Gates: ~4ms. Inference: target <30ms. Margin: ~6ms for HAL jitter.

### Constraint 5: Budget device (Helio G85) compliance
All generated CPU-path code must be benchmarked against these targets:
- Quality gate total: < 4ms on Cortex-A55 @ 2.0GHz (single thread)
- ML inference (TFLite CPU XNNPACK fallback): < 200ms
- ML inference (TFLite GPU delegate, Mali-G57): < 45ms
- Memory: total pipeline heap allocation < 48MB (enforced by OOM guard in session.rs)
Subagent 3 must write benchmark tests that assert these budgets with #[bench] annotations.

### Constraint 6: Post-quantum cryptography (ML-DSA FIPS 204)
The crypto.rs stubs must be completed using the `pqcrypto-dilithium` crate (Rust)
or liboqs FFI bindings. The ML-DSA Level 3 (Dilithium-3) implementation must:
- Generate keypairs deterministically from a 32-byte seed (for test reproducibility)
- Sign the canonical_bytes() output of ScanResult
- Store the private key bytes in a protected memory region (mlock on Linux/Android,
  Secure Enclave API on iOS via CryptoKit, StrongBox on Android via KeyStore API)
- Test: sign → verify round-trip must pass. Signature length must equal 3293 bytes.

---

## SUBAGENT 1 — Rust Core Completion
**Owner**: Claude Opus 4.6
**Dependency**: none (runs in parallel from start)

### Task 1.1 — Cargo.toml
Create `core/Cargo.toml` with:
```toml
[package]
name = "beam-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib", "cdylib"]

[dependencies]
pqcrypto-dilithium = "0.5"   # ML-DSA (Dilithium-3) — FIPS 204
pqcrypto-kyber = "0.8"       # ML-KEM (Kyber-1024) — FIPS 203
pqcrypto-traits = "0.3"
getrandom = { version = "0.2", features = ["js"] }  # WASM entropy

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"              # no unwinding in no_std contexts

[dev-dependencies]
criterion = "0.5"            # benchmarks
```

### Task 1.2 — pipeline.rs
Create `core/src/pipeline.rs` — the FramePipeline struct that:
- Owns a QualityGate and a ScanSession
- Exposes `process_frame(frame: &RawFrame, now_us: u64) -> PipelineResult`
- Returns `PipelineResult::Rejected(QualityReport)` for gate failures
- Returns `PipelineResult::AcceptedForInference` for gate passes (triggers C++ via callback)
- Returns `PipelineResult::Complete(ScanResult)` when session is done
- Handles adaptive gate relaxation (after 60 consecutive failures, relax thresholds 15%)
- Handles timeout (session.is_timed_out() check on every frame)

### Task 1.3 — Complete crypto.rs stubs
Replace stub bodies with real `pqcrypto-dilithium` calls:
```rust
use pqcrypto_dilithium::dilithium3::*;
// PqcSigner::generate() → keypair()
// PqcSigner::sign()     → sign(msg, &sk)
// PqcSigner::verify()   → verify(msg, &sig, &pk)
```
Add `mlock()` call on the private key bytes (via `libc::mlock` on non-WASM targets).

**Verification Artifact**: `cargo build --release --target aarch64-linux-android` exits 0.
Screenshot the terminal output showing successful compilation.

---

## SUBAGENT 2 — Platform Bridges Completion
**Owner**: Claude Opus 4.6
**Dependency**: none (runs in parallel from start)

### Task 2.1 — coreml_bridge.mm (iOS C++/ObjC ML layer)
Create `platform/ios/coreml_bridge.mm`:
- Load a .mlpackage model from the app bundle using `[MLModel modelWithContentsOfURL:]`
- Accept a `CVPixelBufferRef` (already locked by Swift adapter)
- Create `MLFeatureValue` from the CVPixelBuffer — this is the zero-copy path
- Run `[model predictionFromFeatures:options:error:]` with
  `MLPredictionOptions.usesCPUOnly = NO` (ANE dispatch enabled)
- On ANE unavailable (check `MLComputeUnits`), fall back to CPU
- Call `beam_session_push_result()` (Rust FFI) with decoded output
- Expose a plain C API: `beam_coreml_session_t* beam_coreml_create(const char* model_path)`
  and `void beam_coreml_process(beam_coreml_session_t*, CVPixelBufferRef, BeamSessionHandle)`

### Task 2.2 — onnx_bridge.cpp (WASM ML layer)
Create `platform/wasm/onnx_bridge.cpp`:
- Include `onnxruntime_cxx_api.h`
- Accept a raw RGBA uint8 pointer + dimensions (the mandatory copy from WASM adapter)
- Convert RGBA → normalised float tensor (this is the one conversion we cannot avoid on WASM)
- Run inference via `Ort::Session::Run()`
- Use WebGPU execution provider when available: `OrtSessionOptionsAppendExecutionProvider_WebGPU()`
- Fall back to WASM SIMD CPU path
- Expose: `extern "C" void beam_wasm_process_frame(uint8_t* rgba, int w, int h, void* session_handle)`

### Task 2.3 — BeamNativeBridge.kt (Android JNI glue)
Create `platform/android/BeamNativeBridge.kt`:
- Declare all `external fun` JNI methods matching tflite_bridge.cpp exports
- `fun onFrame(yBuffer, uvBuffer, width, height, yStride, uvStride, isNv12, timestampUs)`
  — calls JNI, which calls C++ TFLite bridge, which calls Rust FFI
- `fun createSession(config: SessionConfig): Long` — returns opaque handle
- `fun destroySession(handle: Long)` — releases Rust session
- `fun getResult(handle: Long): ScanResultParcel?` — polls for Complete state
- Load native library: `System.loadLibrary("beam_sdk")`

### Task 2.4 — BeamSDK.swift (iOS public SDK surface)
Create `platform/ios/BeamSDK.swift`:
- Public class `BeamScanner: NSObject`
- Wraps `BeamCameraAdapter` + C coreml_bridge calls via `@_silgen_name`
- Exposes `func startScan(config: BeamScanConfig, completion: @escaping (BeamScanResult) -> Void)`
- Exposes `func stopScan()`
- BeamScanResult mirrors ScanResult from Rust (Codable struct)
- Thread safety: all callbacks on main queue via `DispatchQueue.main.async`

**Verification Artifact**: `xcodebuild -scheme BeamSDK -sdk iphonesimulator build` exits 0.

---

## SUBAGENT 3 — Test Suite
**Owner**: Claude Opus 4.6
**Dependency**: Subagent 1 must complete Task 1.2 before integration tests can run

### Task 3.1 — quality_gate_tests.rs
`tests/quality_gate_tests.rs`:
- Test: sharp 64×64 Y-plane crop → blur_score > 80.0 (passes gate)
- Test: blurred crop (Gaussian-smoothed) → blur_score < 80.0 (fails gate)
- Test: dark frame (mean luma 20) → fails exposure gate
- Test: overexposed frame (P95 luma 248) → fails exposure gate
- Test: identical consecutive crops → motion_score < 0.01
- Test: random vs previous crop → motion_score > 0.12 (fails gate)
- Test: flat colour frame → edge_density < 0.02 (fails boundary gate)
- Test: frame with rectangle → edge_density > 0.08 (passes gate)
- **Critical invariant test**: verify gate short-circuit: a frame that fails Gate::BlurCheck
  must NOT have exposure, motion, or boundary scores computed (check they remain 0.0)

### Task 3.2 — session_state_tests.rs
`tests/session_state_tests.rs`:
- Test: new session → Idle state
- Test: start() → Scanning state
- Test: record_quality_frame() × 3 → returns true (triggers inference)
- Test: record_gate_fail() × 60 → returns true (adaptive relaxation triggered)
- Test: is_timed_out() before 30s → false; after 30s simulated → true
- Test: complete(result) → Complete state, result available

### Task 3.3 — crypto_pqc_tests.rs
`tests/crypto_pqc_tests.rs`:
- Test: PqcSigner::generate() → public_key_bytes().len() == 1952 (Dilithium-3 pk size)
- Test: sign() → signature.len() == 3293 (Dilithium-3 sig size)
- Test: sign → verify round-trip → Ok(true)
- Test: tampered message → verify returns Ok(false) or Err
- Test: ScanResult::canonical_bytes() is deterministic (same fields, same order, same bytes)
- Test: two ScanResults with same fields produce identical canonical_bytes()
- Test: MlKemSession::encapsulate() → ciphertext.len() == 1568
- **Security test**: private key bytes must not appear verbatim in the signature output

### Task 3.4 — ffi_integration_tests.cpp
`tests/ffi_integration_tests.cpp`:
- Include beam_ffi.h (generated from cbindgen or hand-written)
- Test: beam_session_create(default_config) → non-null handle
- Test: beam_gate_create() → non-null handle
- Test: beam_session_start(handle, 0) → state readable as Scanning
- Test: beam_session_push_result() with valid CField array → state == Complete
- Test: beam_session_destroy() on valid handle → no crash (valgrind clean)
- Test: beam_session_destroy(nullptr) → no crash
- **Memory test**: valgrind --leak-check=full on the full create/push/destroy cycle
  must show 0 bytes definitely lost

**Verification Artifact**: `cargo test --release 2>&1 | grep -E "test result|FAILED"` —
screenshot showing all tests pass, 0 failed.

---

## SUBAGENT 4 — Build System and Packaging
**Owner**: Gemini 3 Pro or Claude Sonnet 4.6
**Dependency**: Subagents 1 and 2 must complete before final packaging steps

### Task 4.1 — GitHub Actions CI workflow
Create `.github/workflows/ci.yml`:
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
        with: { targets: "aarch64-linux-android" }
      - name: Install Android NDK r26
        run: sdkmanager "ndk;26.1.10909125"
      - name: Build Rust core (aarch64-android)
        run: cargo build --release --target aarch64-linux-android
        working-directory: beam-sdk/core
      - name: CMake configure + build
        run: |
          cmake -DCMAKE_TOOLCHAIN_FILE=$ANDROID_NDK/build/cmake/android.toolchain.cmake \
                -DANDROID_ABI=arm64-v8a -DANDROID_PLATFORM=android-24 \
                -DBEAM_TARGET=Android ..
          cmake --build . --config Release
        working-directory: beam-sdk/build
      - name: Package AAR
        run: bash beam-sdk/scripts/package_android_aar.sh

  ios-arm64:
    runs-on: macos-15
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: "aarch64-apple-ios" }
      - name: Build Rust core (iOS)
        run: cargo build --release --target aarch64-apple-ios
        working-directory: beam-sdk/core
      - name: xcodebuild
        run: bash beam-sdk/ci/ios_xcodebuild.sh
      - name: Package XCFramework
        run: bash beam-sdk/scripts/package_ios_xcframework.sh

  wasm:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: mymindstorm/setup-emsdk@v14
        with: { version: "3.1.50" }
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: "wasm32-unknown-unknown" }
      - name: Build WASM
        run: bash beam-sdk/ci/wasm_emscripten.sh

  rust-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --release
        working-directory: beam-sdk/core
```

### Task 4.2 — Packaging scripts

`scripts/package_android_aar.sh`:
- Run `cmake --build` for arm64-v8a and armeabi-v7a
- Bundle libbeam_sdk.so + libbeam_core.a into a standard AAR structure
- Include classes.jar with BeamNativeBridge.kt compiled via kotlinc
- Sign with debug keystore if KEYSTORE_PATH not set
- Output: `dist/BeamSDK-{version}-release.aar`

`scripts/package_ios_xcframework.sh`:
- Build for arm64 (device) and x86_64 (simulator)
- `xcodebuild -create-xcframework` combining both slices
- Include Swift overlay module (BeamSDK.swift compiled to .swiftmodule)
- Output: `dist/BeamSDK.xcframework`

`scripts/package_wasm_npm.sh`:
- Run emcmake + emmake
- Copy .wasm + .js glue into `dist/npm/`
- Generate `package.json` with name `@surt/beam-sdk`
- TypeScript declaration file `dist/npm/beam-sdk.d.ts`
- Output: `dist/BeamSDK-{version}.tgz` (ready for `npm publish`)

**Verification Artifact**: Run all three packaging scripts, screenshot the `dist/` directory
listing showing `BeamSDK-0.1.0-release.aar`, `BeamSDK.xcframework/`, and `BeamSDK-0.1.0.tgz`.

---

## SUBAGENT 5 — Documentation
**Owner**: Claude Sonnet 4.6
**Dependency**: Subagents 1–4 complete (so API surfaces are finalised)

### Task 5.1 — Integration Guide
`docs/INTEGRATION_GUIDE.md` must cover:
- Android: add BeamSDK AAR to Gradle, initialise BeamNativeBridge, connect BeamCameraAdapter
- iOS: add BeamSDK.xcframework to Xcode project, instantiate BeamScanner, handle results
- Web/WASM: npm install @surt/beam-sdk, initialise BeamModule, handle mandatory copy caveat
- Each platform section must include a complete minimal code sample (< 20 lines) that compiles

### Task 5.2 — Security Model
`docs/SECURITY_MODEL.md` must cover:
- Why ML-DSA (FIPS 204) was chosen over ECDSA for result signing
- Harvest-now-decrypt-later threat model and why it applies to IDV data
- Key storage: Secure Enclave (iOS) vs StrongBox Keymaster (Android) vs WASM limitation
- Hybrid classical+PQC transition recommendation for 2025–2028
- Transport: ML-KEM-1024 session key + TLS 1.3 + certificate pinning
- Attack surface: adversarial documents, camera feed injection, buffer pool starvation, JNI misuse

### Task 5.3 — API Reference
`docs/API_REFERENCE.md`:
- Rust public API: all pub structs/enums/fns in lib.rs, session.rs, result.rs, crypto.rs
- C FFI API: all #[no_mangle] functions in ffi.rs with full parameter documentation
- Swift API: BeamScanner public interface
- Kotlin API: BeamNativeBridge public interface
- Document every unsafe block with a single-sentence justification

### Task 5.4 — README.md
`README.md`:
- 1-paragraph product summary (what Beam is, what products use it)
- Architecture diagram reference (link to docs/ARCHITECTURE.md — not to be created now)
- Quick-start: 3 commands to build for each platform target
- Platform support matrix table
- Security note: PQC signing enabled by default, how to disable in dev/test
- License and contact

**Verification Artifact**: `markdownlint docs/*.md README.md` exits 0 (no lint errors).

---

## SUCCESS CRITERIA (all must pass before session is marked complete)

| # | Criterion | How verified |
|---|-----------|-------------|
| 1 | Android AAR builds for arm64-v8a | CI job green |
| 2 | iOS XCFramework builds for arm64 | CI job green |
| 3 | WASM module builds via Emscripten | CI job green |
| 4 | All Rust unit tests pass | `cargo test` 0 failures |
| 5 | ML-DSA sign→verify round-trip passes | crypto_pqc_tests.rs |
| 6 | Gate invariant: no GPU invocation on rejected frames | ffi_integration_tests.cpp |
| 7 | Memory: no leaks in FFI cycle | valgrind clean |
| 8 | Quality gate total < 4ms on A55 benchmark | criterion output |
| 9 | All three dist packages exist | package scripts |
| 10 | Markdown docs lint clean | markdownlint |

---

## NOTES FOR ORCHESTRATOR

- If Subagent 2's CoreML bridge fails to compile on the macOS runner due to Xcode version,
  fall back to a stub that returns a hardcoded ScanResult with confidence 0.0 and logs a warning.
  The pipeline must still be architecturally complete even without a real ML model.

- The `pqcrypto-dilithium` crate requires `libclang` at compile time for C FFI generation.
  Ensure the CI Ubuntu runner has `clang` installed: `sudo apt-get install -y clang libclang-dev`.

- WASM Rust target requires: `rustup target add wasm32-unknown-unknown` and
  `cargo install wasm-pack` for optional wasm-pack integration.

- Do not introduce any third-party ML model weights into the repository.
  All inference code must operate on a model loaded at runtime from the host app bundle.
  The C++/ObjC/WASM bridges accept a model path as a parameter — this is intentional.

- The entity graph (IDV result → FaceGuard face embedding → Guardian device fingerprint)
  is assembled server-side by Surt. Beam's responsibility ends at delivering a PQC-signed
  ScanResult to the host app. Do not implement cross-product linking in this SDK layer.
