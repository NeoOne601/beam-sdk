#!/usr/bin/env bash
# ci/wasm_emscripten.sh
# Build the Beam SDK WASM module via Emscripten.
# Requires: emcc on PATH (via setup-emsdk in CI or source emsdk_env.sh locally)
# Exits non-zero on any failure or if the .wasm output is not produced.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${REPO_ROOT}/build_wasm"
OUTPUT_DIR="${REPO_ROOT}/dist/wasm"

echo "=== Beam SDK WASM Build ==="

# ─── Rust WASM target ────────────────────────────────────────────────────────

echo "--- Adding wasm32-unknown-unknown Rust target ---"
rustup target add wasm32-unknown-unknown

# ─── Build Rust core for WASM ────────────────────────────────────────────────

echo "--- Building Rust core for wasm32-unknown-unknown ---"
(cd "${REPO_ROOT}/core" && cargo build --release --target wasm32-unknown-unknown)

# ─── CMake configure via Emscripten ──────────────────────────────────────────

echo "--- CMake configure (Emscripten) ---"
mkdir -p "${BUILD_DIR}"

emcmake cmake \
  -S "${REPO_ROOT}/build" \
  -B "${BUILD_DIR}" \
  -DBEAM_TARGET=WASM \
  -DCMAKE_BUILD_TYPE=Release

# ─── Emmake build ────────────────────────────────────────────────────────────

echo "--- emmake build ---"
emmake make -C "${BUILD_DIR}" -j"$(nproc 2>/dev/null || echo 2)"

# ─── Assert output exists ────────────────────────────────────────────────────

WASM_FILE="${BUILD_DIR}/BeamSDK.wasm"
JS_FILE="${BUILD_DIR}/BeamSDK.js"

if [ ! -f "${WASM_FILE}" ]; then
  echo "ERROR: Expected WASM output not found: ${WASM_FILE}"
  exit 1
fi

if [ ! -f "${JS_FILE}" ]; then
  echo "ERROR: Expected JS loader not found: ${JS_FILE}"
  exit 1
fi

echo "--- WASM output verified ---"
ls -lh "${WASM_FILE}" "${JS_FILE}"

# ─── Copy to dist ────────────────────────────────────────────────────────────

mkdir -p "${OUTPUT_DIR}"
cp "${WASM_FILE}" "${OUTPUT_DIR}/beam_sdk.wasm"
cp "${JS_FILE}"   "${OUTPUT_DIR}/beam_sdk.js"

echo ""
echo "=== WASM build complete ==="
echo "  Output: ${OUTPUT_DIR}"
