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
mkdir -p "${DIST_DIR}" "${AAR_WORK}/jni/arm64-v8a" "${AAR_WORK}/jni/armeabi-v7a" "${AAR_WORK}/jni/x86_64"

# ─── Build native .so for each ABI ───────────────────────────────────────────

build_abi() {
    local ABI="$1"
    local RUST_TARGET="$2"
    local BUILD_DIR="${REPO_ROOT}/build_${ABI}"

    # Check if a prebuilt .so exists from a download-artifact step
    local PREBUILT_SO="${REPO_ROOT}/dist_libs/libbeam_sdk-${ABI}/libbeam_sdk.so"
    if [ -f "${PREBUILT_SO}" ]; then
        echo "Using prebuilt .so for ${ABI} from ${PREBUILT_SO}"
        mkdir -p "${AAR_WORK}/jni/${ABI}"
        cp "${PREBUILT_SO}" "${AAR_WORK}/jni/${ABI}/libbeam_sdk.so"
        echo "  ✓ ${ABI} (prebuilt): $(ls -lh ${PREBUILT_SO} | awk '{print $5}')"
        return 0
    fi

    # Check if rustup has the target installed. If not, skip (unless we are in CI where we fail)
    if command -v rustup &>/dev/null; then
        if ! rustup target list --installed | grep -q "${RUST_TARGET}"; then
            if [ "${CI:-false}" = "true" ]; then
                echo "ERROR: Rust target ${RUST_TARGET} is required in CI but not installed."
                exit 1
            else
                echo "WARNING: Rust target ${RUST_TARGET} not installed. Skipping build for ${ABI}."
                return 0
            fi
        fi
    fi

    # Set up Android cross-compilation environment variables for cargo
    local OS_NAME
    OS_NAME="$(uname -s | tr '[:upper:]' '[:lower:]')"
    local HOST_TAG
    if [ "${OS_NAME}" = "darwin" ]; then
        HOST_TAG="darwin-x86_64"
    elif [ "${OS_NAME}" = "linux" ]; then
        HOST_TAG="linux-x86_64"
    else
        HOST_TAG="windows-x86_64"
    fi
    local TOOLCHAIN="${NDK}/toolchains/llvm/prebuilt/${HOST_TAG}/bin"
    
    local CLANG_PREFIX=""
    local LINKER_VAR=""
    local CC_VAR=""
    local CXX_VAR=""
    local AR_VAR=""
    
    if [ "${ABI}" = "arm64-v8a" ]; then
        CLANG_PREFIX="aarch64-linux-android26-clang"
        LINKER_VAR="CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"
        CC_VAR="CC_aarch64_linux_android"
        CXX_VAR="CXX_aarch64_linux_android"
        AR_VAR="AR_aarch64_linux_android"
    elif [ "${ABI}" = "armeabi-v7a" ]; then
        CLANG_PREFIX="armv7a-linux-androideabi26-clang"
        LINKER_VAR="CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER"
        CC_VAR="CC_armv7_linux_androideabi"
        CXX_VAR="CXX_armv7_linux_androideabi"
        AR_VAR="AR_armv7_linux_androideabi"
    elif [ "${ABI}" = "x86_64" ]; then
        CLANG_PREFIX="x86_64-linux-android26-clang"
        LINKER_VAR="CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER"
        CC_VAR="CC_x86_64_linux_android"
        CXX_VAR="CXX_x86_64_linux_android"
        AR_VAR="AR_x86_64_linux_android"
    fi

    echo "--- Building Rust core for ${RUST_TARGET} ---"
    (
        export PATH="${TOOLCHAIN}:${PATH}"
        if [ -n "${CLANG_PREFIX}" ]; then
            export "${LINKER_VAR}=${CLANG_PREFIX}"
            export "${CC_VAR}=${CLANG_PREFIX}"
            export "${CXX_VAR}=${CLANG_PREFIX}"
            export "${AR_VAR}=llvm-ar"
        fi
        cd "${REPO_ROOT}/core" && cargo build --release --target "${RUST_TARGET}"
    )

    echo "--- CMake build for ${ABI} ---"
    mkdir -p "${BUILD_DIR}"
    (
        export PATH="${TOOLCHAIN}:${PATH}"
        if [ -n "${CLANG_PREFIX}" ]; then
            export "${LINKER_VAR}=${CLANG_PREFIX}"
            export "${CC_VAR}=${CLANG_PREFIX}"
            export "${CXX_VAR}=${CLANG_PREFIX}"
            export "${AR_VAR}=llvm-ar"
        fi
        cmake \
          -S "${REPO_ROOT}/build" \
          -B "${BUILD_DIR}" \
          -DCMAKE_TOOLCHAIN_FILE="${NDK}/build/cmake/android.toolchain.cmake" \
          -DCMAKE_MODULE_PATH="${REPO_ROOT}/build" \
          -DANDROID_ABI="${ABI}" \
          -DANDROID_PLATFORM="android-26" \
          -DBEAM_TARGET=Android \
          -DCMAKE_BUILD_TYPE=Release
        cmake --build "${BUILD_DIR}" --config Release
    )

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
build_abi "x86_64"      "x86_64-linux-android"

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
