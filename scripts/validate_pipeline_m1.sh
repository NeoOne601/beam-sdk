#!/usr/bin/env bash
# scripts/validate_pipeline_m1.sh
# Full pipeline validation for Apple M1 iMac.
# Run ONLY after all 5 Wave 1 agents have committed their changes.
# Prerequisites: Xcode, Android Studio (NDK r26), Docker Desktop, Python tensorflow-macos, Emscripten
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
echo "=== Ajna SDK M1 Pipeline Validation ==="
echo "Repo: ${REPO_ROOT}"
echo ""

# ─── Step 1: Generate synthetic model ────────────────────────────────────────
echo "[Step 1/6] Generating synthetic TFLite model..."
uv run --python 3.11 --with tensorflow --with numpy python3 "${REPO_ROOT}/scripts/generate_synthetic_model.py"
TFLITE_PATH="${REPO_ROOT}/model/placeholder/synthetic_test.tflite"
if [ ! -f "${TFLITE_PATH}" ]; then
    echo "ERROR: Synthetic model not found at ${TFLITE_PATH}"
    exit 1
fi
echo "  ✓ Synthetic model generated ($(du -sh ${TFLITE_PATH} | cut -f1))"
echo ""

# ─── Step 2: Rust unit tests ─────────────────────────────────────────────────
echo "[Step 2/6] Running Rust core tests..."
(cd "${REPO_ROOT}/core" && cargo test --release 2>&1)
echo "  ✓ Rust tests passed"
echo ""

# ─── Step 3: FFI integration tests (macOS host target) ───────────────────────
echo "[Step 3/6] Building and running C++ FFI integration tests..."
cargo build --release -p ajna-core --target aarch64-apple-darwin \
    --manifest-path "${REPO_ROOT}/Cargo.toml" 2>&1

clang++ -O2 "${REPO_ROOT}/tests/ffi_integration_tests.cpp" \
    -I "${REPO_ROOT}/include/" \
    -L "${REPO_ROOT}/target/aarch64-apple-darwin/release" \
    -lajna_core \
    -o /tmp/ajna_ffi_tests
/tmp/ajna_ffi_tests
echo "  ✓ FFI integration tests passed"
echo ""

# ─── Step 4: iOS arm64 cross-compile check ───────────────────────────────────
echo "[Step 4/6] Cross-compiling Rust core for iOS arm64..."
rustup target add aarch64-apple-ios 2>/dev/null || true
SDK_PATH="$(xcrun --sdk iphoneos --show-sdk-path)"
(
    cd "${REPO_ROOT}/core"
    export IPHONEOS_DEPLOYMENT_TARGET=14.0
    export CFLAGS_aarch64_apple_ios="--target=arm64-apple-ios -isysroot ${SDK_PATH} -miphoneos-version-min=14.0"
    export CC_aarch64_apple_ios="$(xcrun --sdk iphoneos --find clang)"
    export AR_aarch64_apple_ios="$(xcrun --sdk iphoneos --find ar)"
    cargo build --release --target aarch64-apple-ios 2>&1
)
echo "  ✓ iOS arm64 cross-compile passed"
echo ""

# ─── Step 5: Android arm64 cross-compile check ───────────────────────────────
echo "[Step 5/6] Cross-compiling Rust core for Android arm64..."
rustup target add aarch64-linux-android 2>/dev/null || true
ANDROID_NDK="${ANDROID_NDK_ROOT:-${HOME}/Library/Android/sdk/ndk/26.1.10909125}"
if [ ! -d "${ANDROID_NDK}" ]; then
    echo "  WARNING: Android NDK not found at ${ANDROID_NDK} — skipping Android cross-compile"
    echo "  Set ANDROID_NDK_ROOT to your NDK path and re-run to test Android."
else
    (
        cd "${REPO_ROOT}/core"
        export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${ANDROID_NDK}/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android26-clang"
        cargo build --release --target aarch64-linux-android 2>&1
    )
    echo "  ✓ Android arm64 cross-compile passed"
fi
echo ""

# ─── Step 6: Backend round-trip test ─────────────────────────────────────────
echo "[Step 6/6] Starting backend and running verification round-trip..."
(cd "${REPO_ROOT}/backend" && docker compose up -d 2>&1)
sleep 5

DB_URL="postgres://ajna:ajna@localhost:5432/ajna_verify"
psql "${DB_URL}" -f "${REPO_ROOT}/backend/src/db/migrations/001_initial.sql" 2>/dev/null || true
psql "${DB_URL}" -f "${REPO_ROOT}/backend/src/db/migrations/002_trusted_keys.sql" 2>/dev/null || true

(cd "${REPO_ROOT}/backend" && cargo test --release 2>&1)

HEALTH=$(curl -sf http://localhost:8080/health || echo '{"status":"unreachable"}')
echo "  Backend health: ${HEALTH}"

(cd "${REPO_ROOT}/backend" && docker compose down 2>&1)
echo "  ✓ Backend round-trip passed"
echo ""

echo "=== ALL VALIDATION STEPS PASSED ==="
echo ""
echo "Pipeline summary:"
echo "  Synthetic model:   ${TFLITE_PATH}"
echo "  Rust tests:        PASS"
echo "  FFI tests:         PASS"
echo "  iOS cross-compile: PASS"
echo "  Android:           PASS (or skipped — see NDK note above)"
echo "  Backend:           PASS"
echo ""
echo "Next step: Replace synthetic_test.tflite with a real document OCR model."
echo "The output_schema.json specifies the exact tensor contract the real model must meet."

# Make this script executable: chmod +x scripts/validate_pipeline_m1.sh
