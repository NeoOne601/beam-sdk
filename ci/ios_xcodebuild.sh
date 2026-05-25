#!/usr/bin/env bash
# ci/ios_xcodebuild.sh
# Build BeamSDK for iOS device (arm64) and simulator (x86_64 + arm64).
# Exits non-zero on any failure.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCHEME="BeamSDK"

echo "=== Beam SDK iOS Build ==="
echo "Repository: ${REPO_ROOT}"

# ─── Generate Xcode Projects using CMake ──────────────────────────────────────

echo "--- Generating Xcode project for iOS Device (arm64) ---"
mkdir -p "${REPO_ROOT}/build_ios_device"
cmake -G Xcode \
  -S "${REPO_ROOT}/build" \
  -B "${REPO_ROOT}/build_ios_device" \
  -DCMAKE_SYSTEM_NAME=iOS \
  -DCMAKE_OSX_SYSROOT=iphoneos \
  -DBEAM_TARGET=iOS

echo "--- Generating Xcode project for iOS Simulator (x86_64) ---"
mkdir -p "${REPO_ROOT}/build_ios_sim"
cmake -G Xcode \
  -S "${REPO_ROOT}/build" \
  -B "${REPO_ROOT}/build_ios_sim" \
  -DCMAKE_SYSTEM_NAME=iOS \
  -DCMAKE_OSX_SYSROOT=iphonesimulator \
  -DBEAM_TARGET=iOS

# ─── Device build (arm64) ────────────────────────────────────────────────────

echo ""
echo "--- Building for iOS device (arm64) ---"
xcodebuild \
  -project        "${REPO_ROOT}/build_ios_device/beam_sdk.xcodeproj" \
  -scheme         "${SCHEME}" \
  -sdk            iphoneos \
  -configuration  Release \
  build \
  ONLY_ACTIVE_ARCH=NO \
  BUILD_DIR="${REPO_ROOT}/build_ios/device" \
  | tee /tmp/xcodebuild_device.log

if [ ${PIPESTATUS[0]} -ne 0 ]; then
  echo "ERROR: Device build failed. Log at /tmp/xcodebuild_device.log"
  exit 1
fi

# ─── Simulator build (x86_64 + arm64 for Apple Silicon Macs) ────────────────

echo ""
echo "--- Building for iOS simulator ---"
xcodebuild \
  -project        "${REPO_ROOT}/build_ios_sim/beam_sdk.xcodeproj" \
  -scheme         "${SCHEME}" \
  -sdk            iphonesimulator \
  -configuration  Release \
  build \
  ONLY_ACTIVE_ARCH=NO \
  BUILD_DIR="${REPO_ROOT}/build_ios/simulator" \
  | tee /tmp/xcodebuild_sim.log

if [ ${PIPESTATUS[0]} -ne 0 ]; then
  echo "ERROR: Simulator build failed. Log at /tmp/xcodebuild_sim.log"
  exit 1
fi

echo ""
echo "=== iOS build complete ==="
echo "  Device:    ${REPO_ROOT}/build_ios/device"
echo "  Simulator: ${REPO_ROOT}/build_ios/simulator"
