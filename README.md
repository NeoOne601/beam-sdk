# Beam SDK

**Beam** is Surt AI's cross-platform document scanning SDK. It provides the core ML inference pipeline, quality-gated frame processing, and post-quantum signed result delivery for three Surt products: **IDV** (identity verification), **Guardian** (device integrity), and **FaceGuard** (facial authentication). Beam runs as native C++ at the ML runtime boundary and Rust for all business logic.

---

## Platform Support

| Platform | ABI | ML Backend | Status |
|----------|-----|-----------|--------|
| iOS | arm64 | CoreML (ANE + CPU fallback) | ✅ |
| Android | arm64-v8a | TFLite GPU delegate (Mali/Adreno) | ✅ |
| Android | armeabi-v7a | TFLite NNAPI + CPU fallback | ✅ |
| Web | WASM | ONNX Runtime (WebGPU + SIMD fallback) | ✅ |

---

## Quick Start

### Android (arm64-v8a)

```bash
# Install Rust Android target
rustup target add aarch64-linux-android

# Build Rust core
cd core && cargo build --release --target aarch64-linux-android

# CMake build
mkdir build_output && cd build_output
cmake -DCMAKE_TOOLCHAIN_FILE=$ANDROID_NDK/build/cmake/android.toolchain.cmake \
      -DANDROID_ABI=arm64-v8a -DANDROID_PLATFORM=android-24 \
      -DBEAM_TARGET=Android ../build
cmake --build . --config Release

# Package AAR
bash scripts/package_android_aar.sh
```

### iOS (arm64)

```bash
rustup target add aarch64-apple-ios
cd core && cargo build --release --target aarch64-apple-ios
bash ci/ios_xcodebuild.sh
bash scripts/package_ios_xcframework.sh
```

### Web (WASM)

```bash
rustup target add wasm32-unknown-unknown
bash ci/wasm_emscripten.sh
bash scripts/package_wasm_npm.sh
```

---

## Security

Beam signs every `ScanResult` with **ML-DSA Level 3** (CRYSTALS-Dilithium, NIST FIPS 204, August 2024) by default. This provides 128-bit post-quantum security against harvest-now-decrypt-later attacks.

**To disable PQC signing in development builds** (reduces latency by ~20ms on budget devices):

```kotlin
// Android
val config = SessionConfig(pqcSignResult = false)
bridge.nativeStartSession(handle, timestampUs)
```

```swift
// iOS
scanner.startScan(config: BeamScanConfig(pqcSignResult: false)) { ... }
```

```typescript
// WASM
Beam.beam_session_create({ ..., pqcSignResult: false });
```

> ⚠️ **Never disable PQC signing in production.** The default `pqcSignResult: true` is the required configuration for Surt compliance verification.

---

## Documentation

- [docs/INTEGRATION_GUIDE.md](docs/INTEGRATION_GUIDE.md) — Platform-specific integration instructions
- [docs/SECURITY_MODEL.md](docs/SECURITY_MODEL.md) — PQC threat model, key storage, attack surface
- [docs/API_REFERENCE.md](docs/API_REFERENCE.md) — Full Rust/Swift/Kotlin API reference

---

## Architecture

```mermaid
flowchart TD
    Camera[Camera HAL] -->|Raw Frames| Adapter[Swift/Kotlin Adapter]
    Adapter -->|Frame Info| Gates[Quality Gates <br/> Rust / CPU, &lt; 4ms]
    Gates -->|Gate::Accepted only| Bridge[C++ ML Bridge]
    
    subgraph Execution Engines
        Bridge -->|iOS| CoreML[CoreML <br/> ANE + CPU fallback]
        Bridge -->|Android| TFLite[TFLite <br/> GPU delegate / NNAPI]
        Bridge -->|Web| ONNX[ONNX Runtime <br/> WebGPU + SIMD]
    end
    
    CoreML & TFLite & ONNX --> Push[beam_session_push_result]
    Push --> Sign[Rust PQC Sign <br/> ML-DSA Level 3]
    Sign --> Result[ScanResult]

    classDef default fill:#1f2937,stroke:#4b5563,stroke-width:1px,color:#f3f4f6;
    classDef highlight fill:#1e3a8a,stroke:#3b82f6,stroke-width:2px,color:#eff6ff;
    classDef gate fill:#065f46,stroke:#10b981,stroke-width:2px,color:#ecfdf5;
    classDef engine fill:#374151,stroke:#6b7280,stroke-width:1px,color:#f3f4f6;
    
    class Gates gate;
    class Bridge,Camera,Adapter highlight;
    class CoreML,TFLite,ONNX engine;
```

The C++ GPU layer is **never invoked** on a frame rejected by the quality gates.

---

## License

<!-- TODO: Add license -->
