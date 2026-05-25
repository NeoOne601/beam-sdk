#!/usr/bin/env bash
# ci/ios_xcodebuild.sh
# Build BeamSDK for iOS device (arm64) and simulator (x86_64 + arm64).
# Exits non-zero on any failure.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SCHEME="BeamSDK"

echo "=== Beam SDK iOS Build ==="
echo "Repository: ${REPO_ROOT}"

# ─── Device build (arm64) ────────────────────────────────────────────────────

echo ""
echo "--- Building for iOS device (arm64) ---"
xcodebuild \
  -scheme         "${SCHEME}" \
  -sdk            iphoneos \
  -arch           arm64 \
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
  -scheme         "${SCHEME}" \
  -sdk            iphonesimulator \
  -arch           x86_64 \
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
