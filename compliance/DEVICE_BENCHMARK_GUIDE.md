# Ajna Verify SDK — Device Benchmark Guide

## Purpose

This guide provides standard procedures for benchmarking the Ajna Verify SDK on target devices. Results inform quality gate threshold tuning and determine whether a device meets the performance budget.

## Performance Budget

| Component | Budget | Reference Device |
|-----------|--------|-----------------|
| Quality gates (all 4) | < 4 ms | MediaTek Helio G85 (Cortex-A55) |
| TFLite GPU inference | < 45 ms | Mali-G57 MC2 (GPU delegate) |
| TFLite CPU inference | < 200 ms | Cortex-A55 × 2 threads (XNNPACK) |
| CoreML inference (ANE) | < 30 ms | Apple A14 Neural Engine |
| ONNX WASM inference | < 150 ms | Chrome 120+ on desktop |
| Pipeline heap (total) | < 48 MB | Including model weights |
| Frame budget at 25fps | 40 ms | 4ms gates + 30ms inference + 6ms margin |

## Benchmark Procedure

### 1. Quality Gate Benchmarks (Rust)

```bash
cd core
cargo bench --bench pipeline_bench
```

Key metrics:
- `blur_gate_1080p` — Laplacian variance on 64×64 crop
- `full_gate_pipeline_1080p` — All 4 gates sequential
- `rejected_at_blur_1080p` — Short-circuit path (must be < 500µs)

### 2. PQC Crypto Benchmarks (Rust)

```bash
cd core
cargo bench --bench crypto_bench
```

Key metrics:
- `ml_dsa_keygen_level3` — Key generation
- `ml_dsa_sign_256b` — Signing 256 bytes
- `ml_dsa_verify` — Signature verification
- `ml_kem_encapsulate` — KEM encapsulation

### 3. Android On-Device Benchmark

**Prerequisites**: Debug build installed, `adb` connected.

```bash
# Run Rust benchmarks via JNI (requires benchmark harness in sample app)
adb shell am instrument -w com.ajna.sample/.AjnaBenchmarkRunner

# Memory profiling
adb shell dumpsys meminfo com.ajna.sample | grep "TOTAL"

# GPU inference timing (logcat)
adb logcat -s AjnaTFLite:V | grep "inference_ms"
```

### 4. iOS On-Device Benchmark

Use Xcode Instruments:
1. **Time Profiler**: Attach to AjnaVerifySample, run scan flow
2. **Allocations**: Track peak heap during pipeline
3. **Metal System Trace**: CoreML GPU/ANE utilisation

### 5. WASM Browser Benchmark

```javascript
// In browser console after loading the web sample
const start = performance.now();
// Process 100 frames
for (let i = 0; i < 100; i++) {
    await processFrame(testImageData);
}
const elapsed = performance.now() - start;
console.log(`Average frame time: ${elapsed / 100}ms`);
```

## Reporting Template

```
Device: [model name]
SoC: [chipset]
OS: [version]
Build: [debug/release]

Quality Gates (avg/p99):
  blur_gate: X.XX ms / X.XX ms
  full_pipeline: X.XX ms / X.XX ms

Inference:
  GPU: X.XX ms (delegate: [GPU/NNAPI/ANE])
  CPU: X.XX ms (threads: N)

Memory:
  Peak heap: XX MB
  Model size: XX MB

PQC:
  keygen: X.XX ms
  sign: X.XX ms
  verify: X.XX ms

PASS/FAIL: [meets budget?]
```

## Minimum Supported Devices

| Platform | Minimum | Target |
|----------|---------|--------|
| Android | Helio G85 (Redmi Note 9) | Snapdragon 778G |
| iOS | A14 (iPhone 12) | A16 (iPhone 14 Pro) |
| WASM | Chrome 120 desktop | Chrome 120 + WebGPU |
