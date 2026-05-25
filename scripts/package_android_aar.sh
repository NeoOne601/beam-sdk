#!/usr/bin/env bash
# scripts/package_android_aar.sh
# Package the Beam SDK as an Android AAR for distribution.
#
# Output: dist/BeamSDK-0.1.0-release.aar
#
# AAR structure:
#   jni/arm64-v8a/libbeam_sdk.so
#   jni/armeabi-v7a/libbeam_sdk.so
#   classes.jar  (compiled BeamNativeBridge.kt)
#   AndroidManifest.xml
#
# Prerequisites:
#   - Android NDK r26 at $ANDROID_NDK
#   - cmake, kotlinc on PATH
#   - cargo with aarch64-linux-android and armv7-linux-androideabi targets

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"
AAR_WORK="${DIST_DIR}/.aar_work"
AAR_OUT="${DIST_DIR}/BeamSDK-0.1.0-release.aar"
NDK="${ANDROID_NDK:-${ANDROID_NDK_ROOT:-/opt/android/ndk}}"
VERSION="0.1.0"

echo "=== Beam SDK Android AAR Packaging ==="
echo "NDK: ${NDK}"
mkdir -p "${DIST_DIR}" "${AAR_WORK}/jni/arm64-v8a" "${AAR_WORK}/jni/armeabi-v7a"

# ─── Build native .so for each ABI ───────────────────────────────────────────

build_abi() {
    local ABI="$1"
    local RUST_TARGET="$2"
    local BUILD_DIR="${REPO_ROOT}/build_${ABI}"

    echo "--- Building Rust core for ${RUST_TARGET} ---"
    (cd "${REPO_ROOT}/core" && cargo build --release --target "${RUST_TARGET}")

    echo "--- CMake build for ${ABI} ---"
    mkdir -p "${BUILD_DIR}"
    cmake \
      -S "${REPO_ROOT}/build" \
      -B "${BUILD_DIR}" \
      -DCMAKE_TOOLCHAIN_FILE="${NDK}/build/cmake/android.toolchain.cmake" \
      -DANDROID_ABI="${ABI}" \
      -DANDROID_PLATFORM="android-24" \
      -DBEAM_TARGET=Android \
      -DCMAKE_BUILD_TYPE=Release
    cmake --build "${BUILD_DIR}" --config Release

    local SO="${BUILD_DIR}/libbeam_sdk.so"
    if [ ! -f "${SO}" ]; then
        echo "ERROR: libbeam_sdk.so not found for ${ABI}"
        exit 1
    fi
    cp "${SO}" "${AAR_WORK}/jni/${ABI}/libbeam_sdk.so"
    echo "  ✓ ${ABI}: $(ls -lh ${SO} | awk '{print $5}')"
}

build_abi "arm64-v8a"   "aarch64-linux-android"
build_abi "armeabi-v7a" "armv7-linux-androideabi"

# ─── Compile BeamNativeBridge.kt → classes.jar ───────────────────────────────

KOTLIN_SRC="${REPO_ROOT}/platform/android/BeamNativeBridge.kt"
KOTLIN_OUT="${AAR_WORK}/kotlin_classes"
mkdir -p "${KOTLIN_OUT}"

echo "--- Compiling Kotlin bridge ---"
if command -v kotlinc &>/dev/null; then
    kotlinc "${KOTLIN_SRC}" -d "${KOTLIN_OUT}" 2>/dev/null || true
    # Pack into classes.jar
    (cd "${KOTLIN_OUT}" && jar cf "${AAR_WORK}/classes.jar" .)
else
    echo "WARNING: kotlinc not found; creating empty classes.jar"
    (cd "${AAR_WORK}" && echo "" | jar cf classes.jar -C . .)
fi

# ─── AndroidManifest.xml ─────────────────────────────────────────────────────

cat > "${AAR_WORK}/AndroidManifest.xml" <<'EOF'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="ai.surt.beam"
    android:versionCode="1"
    android:versionName="0.1.0">
</manifest>
EOF

# ─── Create AAR (zip) ────────────────────────────────────────────────────────

echo "--- Creating AAR ---"
(cd "${AAR_WORK}" && zip -r "${AAR_OUT}" .)

# ─── Sign with debug keystore if KEYSTORE_PATH not set ───────────────────────

if [ -z "${KEYSTORE_PATH:-}" ]; then
    echo "--- Signing with debug keystore ---"
    DEBUG_KS="${HOME}/.android/debug.keystore"
    if [ -f "${DEBUG_KS}" ] && command -v jarsigner &>/dev/null; then
        jarsigner -keystore "${DEBUG_KS}" \
                  -storepass android \
                  -keypass   android \
                  "${AAR_OUT}" androiddebugkey 2>/dev/null || true
    else
        echo "WARNING: debug keystore or jarsigner not available; AAR is unsigned"
    fi
else
    echo "--- Signing with provided keystore ---"
    jarsigner -keystore "${KEYSTORE_PATH}" \
              -storepass "${KEYSTORE_PASS:-changeit}" \
              "${AAR_OUT}" "${KEY_ALIAS:-beam}"
fi

echo ""
echo "=== AAR packaging complete ==="
echo "  Output: ${AAR_OUT}"
ls -lh "${AAR_OUT}"
