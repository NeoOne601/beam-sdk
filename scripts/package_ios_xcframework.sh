#!/usr/bin/env bash
# scripts/package_ios_xcframework.sh
# Package the Ajna SDK (Rust core) as an XCFramework for iOS distribution.
#
# Output: dist/AjnaSDK.xcframework/  — the compiled Rust core static library
# (libajna_core.a) per slice, plus the C FFI header (ajna_ffi.h) + module map
# an iOS app imports to call the engine. The SDK *is* the Rust core; no
# CMake / CoreML Xcode build is required (the CoreML ML bridge is separate).
#
# Prerequisites: Xcode (xcodebuild) + `rustup target add aarch64-apple-ios
# aarch64-apple-ios-sim`.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"
XCF_OUT="${DIST_DIR}/AjnaSDK.xcframework"
DEVICE_TARGET="aarch64-apple-ios"
SIM_TARGET="aarch64-apple-ios-sim"

echo "=== Ajna SDK iOS XCFramework Packaging ==="
mkdir -p "${DIST_DIR}"

# ─── Compile the Rust core for both iOS slices (-j 2 per M1 RAM rule) ──────────
for T in "${DEVICE_TARGET}" "${SIM_TARGET}"; do
  echo "--- cargo build --release --target ${T} -p ajna-core ---"
  ( cd "${REPO_ROOT}" && cargo build --release --target "${T}" -j 2 -p ajna-core )
done

DEVICE_LIB="${REPO_ROOT}/target/${DEVICE_TARGET}/release/libajna_core.a"
SIM_LIB="${REPO_ROOT}/target/${SIM_TARGET}/release/libajna_core.a"

# ─── Stage headers (module map + C FFI header) ────────────────────────────────
HDR_DIR="${DIST_DIR}/ajna_headers"
rm -rf "${HDR_DIR}"; mkdir -p "${HDR_DIR}"
cp "${REPO_ROOT}/include/ajna_ffi.h" "${HDR_DIR}/ajna_ffi.h"
cat > "${HDR_DIR}/module.modulemap" <<'EOF'
module AjnaSDK {
    header "ajna_ffi.h"
    export *
}
EOF

# ─── Assemble the XCFramework ─────────────────────────────────────────────────
rm -rf "${XCF_OUT}"
xcodebuild -create-xcframework \
  -library "${DEVICE_LIB}" -headers "${HDR_DIR}" \
  -library "${SIM_LIB}"    -headers "${HDR_DIR}" \
  -output  "${XCF_OUT}"

echo ""
echo "=== XCFramework packaging complete: ${XCF_OUT} ==="
ls -R "${XCF_OUT}" | head -30
