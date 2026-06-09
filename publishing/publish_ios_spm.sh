#!/usr/bin/env bash
# publish_ios_spm.sh — Generate Swift Package Manager manifest for Beam Verify iOS SDK.
# Usage: ./publish_ios_spm.sh <version>
# Prerequisites:
#   - XCFramework built by scripts/package_ios_xcframework.sh
#   - GITHUB_TOKEN env var set
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"
XCFRAMEWORK_PATH="build_output/BeamVerify.xcframework"
DIST_DIR="build_output/spm"

if [ ! -d "${XCFRAMEWORK_PATH}" ]; then
    echo "ERROR: XCFramework not found at ${XCFRAMEWORK_PATH}"
    echo "Run scripts/package_ios_xcframework.sh first."
    exit 1
fi

mkdir -p "${DIST_DIR}"

# Zip the XCFramework
ZIP_FILE="${DIST_DIR}/BeamVerify-${VERSION}.xcframework.zip"
cd build_output && zip -r "../${ZIP_FILE}" BeamVerify.xcframework && cd ..
CHECKSUM=$(swift package compute-checksum "${ZIP_FILE}" 2>/dev/null || shasum -a 256 "${ZIP_FILE}" | awk '{print $1}')

# Generate Package.swift
cat > "${DIST_DIR}/Package.swift" <<EOF
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "BeamVerify",
    platforms: [
        .iOS(.v14)
    ],
    products: [
        .library(
            name: "BeamVerify",
            targets: ["BeamVerify"]
        ),
    ],
    targets: [
        .binaryTarget(
            name: "BeamVerify",
            url: "https://github.com/surt-ai/beam-sdk/releases/download/v${VERSION}/BeamVerify-${VERSION}.xcframework.zip",
            checksum: "${CHECKSUM}"
        ),
    ]
)
EOF

echo "✓ SPM package generated at ${DIST_DIR}/Package.swift"
echo "  Checksum: ${CHECKSUM}"
echo ""
echo "Integration instructions:"
echo "  1. Upload ${ZIP_FILE} to GitHub release v${VERSION}"
echo "  2. Copy ${DIST_DIR}/Package.swift to repo root"
echo "  3. Tag release: git tag v${VERSION} && git push --tags"
echo "  4. Users add: .package(url: \"https://github.com/surt-ai/beam-sdk\", from: \"${VERSION}\")"
