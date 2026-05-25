#!/usr/bin/env bash
# scripts/package_ios_xcframework.sh
# Package the Beam SDK as an XCFramework for iOS distribution.
#
# Output: dist/BeamSDK.xcframework/
#
# Prerequisites:
#   - Xcode with xcodebuild on PATH
#   - ci/ios_xcodebuild.sh must have been run or will be invoked here
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"
XCF_OUT="${DIST_DIR}/BeamSDK.xcframework"
BUILD_IOS="${REPO_ROOT}/build_ios"
SCHEME="BeamSDK"

echo "=== Beam SDK iOS XCFramework Packaging ==="
mkdir -p "${DIST_DIR}"

# ─── Invoke CI build script ───────────────────────────────────────────────────

echo "--- Running iOS build script ---"
bash "${REPO_ROOT}/ci/ios_xcodebuild.sh"

# ─── Locate built products ────────────────────────────────────────────────────

DEVICE_ARCHIVE="${REPO_ROOT}/build_ios_device/Release-iphoneos/${SCHEME}.framework"
SIM_ARCHIVE="${REPO_ROOT}/build_ios_sim/Release-iphonesimulator/${SCHEME}.framework"

# Fall back to .a if framework is not present (static lib build)
DEVICE_LIB="${REPO_ROOT}/build_ios_device/Release-iphoneos/libBeamSDK.a"
SIM_LIB="${REPO_ROOT}/build_ios_sim/Release-iphonesimulator/libBeamSDK.a"

# ─── Create XCFramework ───────────────────────────────────────────────────────

rm -rf "${XCF_OUT}"

if [ -d "${DEVICE_ARCHIVE}" ] && [ -d "${SIM_ARCHIVE}" ]; then
    echo "--- Creating XCFramework from .framework archives ---"
    xcodebuild -create-xcframework \
        -framework "${DEVICE_ARCHIVE}" \
        -framework "${SIM_ARCHIVE}" \
        -output    "${XCF_OUT}"
elif [ -f "${DEVICE_LIB}" ] && [ -f "${SIM_LIB}" ]; then
    echo "--- Creating XCFramework from static libraries ---"
    xcodebuild -create-xcframework \
        -library "${DEVICE_LIB}" \
        -library "${SIM_LIB}" \
        -output  "${XCF_OUT}"
else
    echo "ERROR: No framework or library products found in build_ios/"
    echo "  Looked for:"
    echo "    ${DEVICE_ARCHIVE}"
    echo "    ${SIM_ARCHIVE}"
    echo "    ${DEVICE_LIB}"
    echo "    ${SIM_LIB}"
    exit 1
fi

echo ""
echo "=== XCFramework packaging complete ==="
echo "  Output: ${XCF_OUT}"
ls -la "${XCF_OUT}"
