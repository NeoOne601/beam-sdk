#!/usr/bin/env bash
# scripts/package_android_aar.sh
# Package the Ajna SDK (Rust core) as an Android AAR.
#
# Output: dist/AjnaSDK-0.1.0-release.aar
#   jni/<abi>/libajna_sdk.so   (the compiled Rust core, one per ABI)
#   classes.jar                (compiled AjnaSDK.kt JNI bridge, if kotlinc present)
#   AndroidManifest.xml
#   R.txt (empty)
#
# The SDK *is* the Rust core (business logic, quality gates, PQC signing, FFI).
# No CMake / TFLite C++ build required — the app links libajna_sdk.so via JNI.
#
# Prerequisites:
#   ANDROID_NDK set to an NDK with the aarch64/armv7/x86_64 android26 clangs.
#   rustup targets: aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"
AAR_WORK="${DIST_DIR}/.aar_work"
AAR_OUT="${DIST_DIR}/AjnaSDK-0.1.0-release.aar"
NDK="${ANDROID_NDK:-${ANDROID_NDK_ROOT:-/opt/android/ndk}}"

HOST_TAG="darwin-x86_64"
[ "$(uname -s)" = "Linux" ] && HOST_TAG="linux-x86_64"
TC="${NDK}/toolchains/llvm/prebuilt/${HOST_TAG}/bin"

echo "=== Ajna SDK Android AAR Packaging ==="
echo "NDK: ${NDK}"
rm -rf "${AAR_WORK}"; mkdir -p "${DIST_DIR}"

# ABI → (rust target, clang prefix, linker env var, cc env var)
build_abi() {
  local ABI="$1" RUST_TARGET="$2" CLANG="$3" LINKER_VAR="$4" CC_VAR="$5"
  mkdir -p "${AAR_WORK}/jni/${ABI}"
  echo "--- cargo build --release --target ${RUST_TARGET} -p ajna-core ---"
  (
    export PATH="${TC}:${PATH}"
    export "${LINKER_VAR}=${CLANG}" "${CC_VAR}=${CLANG}"
    cd "${REPO_ROOT}" && cargo build --release --target "${RUST_TARGET}" -j 2 -p ajna-core
  )
  # The JNI class calls System.loadLibrary("ajna_sdk"); ship the core under that name.
  cp "${REPO_ROOT}/target/${RUST_TARGET}/release/libajna_core.so" \
     "${AAR_WORK}/jni/${ABI}/libajna_sdk.so"
  echo "  ✓ ${ABI}"
}

build_abi arm64-v8a   aarch64-linux-android    aarch64-linux-android26-clang \
          CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER CC_aarch64_linux_android
build_abi armeabi-v7a armv7-linux-androideabi  armv7a-linux-androideabi26-clang \
          CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER CC_armv7_linux_androideabi
build_abi x86_64      x86_64-linux-android      x86_64-linux-android26-clang \
          CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER CC_x86_64_linux_android

# ─── classes.jar from the Kotlin JNI bridge (optional) ────────────────────────
mkdir -p "${AAR_WORK}/kotlin_classes"
if command -v kotlinc &>/dev/null; then
  echo "--- compiling AjnaSDK.kt → classes.jar ---"
  kotlinc "${REPO_ROOT}/platform/android/AjnaSDK.kt" -d "${AAR_WORK}/kotlin_classes" 2>/dev/null || true
fi
(cd "${AAR_WORK}/kotlin_classes" && jar cf "${AAR_WORK}/classes.jar" . 2>/dev/null || jar cf "${AAR_WORK}/classes.jar" -C "${AAR_WORK}/kotlin_classes" .)

# ─── AAR metadata ─────────────────────────────────────────────────────────────
cat > "${AAR_WORK}/AndroidManifest.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="com.ajna.sdk" android:versionCode="1" android:versionName="0.1.0">
    <uses-sdk android:minSdkVersion="26" />
</manifest>
EOF
: > "${AAR_WORK}/R.txt"

# ─── Zip the AAR ──────────────────────────────────────────────────────────────
rm -f "${AAR_OUT}"
(cd "${AAR_WORK}" && zip -rq "${AAR_OUT}" AndroidManifest.xml classes.jar R.txt jni)

echo ""
echo "=== AAR packaging complete ==="
ls -lh "${AAR_OUT}"
unzip -l "${AAR_OUT}" | grep -E "\.so|classes.jar|Manifest"
