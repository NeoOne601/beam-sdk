#!/usr/bin/env bash
# publish_android_maven.sh — Publish Beam Verify Android SDK to Maven (GitHub Packages).
# Usage: ./publish_android_maven.sh <version>
# Prerequisites:
#   - GITHUB_TOKEN env var set with write:packages scope
#   - AAR built by scripts/package_android_aar.sh
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"
AAR_PATH="build_output/beam-verify-${VERSION}.aar"
GROUP_ID="ai.surt.beam"
ARTIFACT_ID="beam-verify"
REPO_URL="https://maven.pkg.github.com/surt-ai/beam-sdk"

if [ ! -f "${AAR_PATH}" ]; then
    echo "ERROR: AAR not found at ${AAR_PATH}"
    echo "Run scripts/package_android_aar.sh first."
    exit 1
fi

echo "Publishing ${ARTIFACT_ID}:${VERSION} to GitHub Packages..."

# Generate POM
POM_FILE="build_output/beam-verify-${VERSION}.pom"
cat > "${POM_FILE}" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>${GROUP_ID}</groupId>
    <artifactId>${ARTIFACT_ID}</artifactId>
    <version>${VERSION}</version>
    <packaging>aar</packaging>
    <name>Beam Verify SDK</name>
    <description>Post-quantum cryptographically signed identity verification SDK for Android</description>
    <url>https://github.com/surt-ai/beam-sdk</url>
    <licenses>
        <license>
            <name>Proprietary</name>
            <url>https://surt.ai/licenses/beam-verify</url>
        </license>
    </licenses>
</project>
EOF

# Publish to GitHub Packages
curl -sf -X PUT \
    -u "surt-ai:${GITHUB_TOKEN}" \
    -H "Content-Type: application/octet-stream" \
    --data-binary "@${AAR_PATH}" \
    "${REPO_URL}/${GROUP_ID//./\/}/${ARTIFACT_ID}/${VERSION}/${ARTIFACT_ID}-${VERSION}.aar"

curl -sf -X PUT \
    -u "surt-ai:${GITHUB_TOKEN}" \
    -H "Content-Type: application/xml" \
    --data-binary "@${POM_FILE}" \
    "${REPO_URL}/${GROUP_ID//./\/}/${ARTIFACT_ID}/${VERSION}/${ARTIFACT_ID}-${VERSION}.pom"

echo "✓ Published ${GROUP_ID}:${ARTIFACT_ID}:${VERSION}"
echo ""
echo "Integration instructions:"
echo "  repositories { maven { url 'https://maven.pkg.github.com/surt-ai/beam-sdk' } }"
echo "  dependencies { implementation '${GROUP_ID}:${ARTIFACT_ID}:${VERSION}' }"
