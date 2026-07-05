#!/usr/bin/env bash
# publish_npm.sh — Publish Ajna Verify WASM SDK to npm.
# Usage: ./publish_npm.sh <version>
# Prerequisites:
#   - WASM build completed by ci/wasm_emscripten.sh
#   - NPM_TOKEN env var set
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"
DIST_DIR="build_output/npm"
WASM_DIR="build_output/wasm"

if [ ! -d "${WASM_DIR}" ]; then
    echo "ERROR: WASM build output not found at ${WASM_DIR}"
    echo "Run ci/wasm_emscripten.sh first."
    exit 1
fi

mkdir -p "${DIST_DIR}"
cp -r "${WASM_DIR}"/* "${DIST_DIR}/"

# Generate package.json
cat > "${DIST_DIR}/package.json" <<EOF
{
  "name": "@ajna-ai/ajna-verify",
  "version": "${VERSION}",
  "description": "Post-quantum cryptographically signed identity verification SDK for the web",
  "main": "ajna_verify.js",
  "types": "ajna_verify.d.ts",
  "files": [
    "ajna_verify.js",
    "ajna_verify.wasm",
    "ajna_verify.d.ts",
    "README.md"
  ],
  "keywords": ["identity-verification", "pqc", "wasm", "document-scanning", "ml-dsa"],
  "license": "SEE LICENSE IN LICENSE",
  "author": "Ajna AI <sdk@ajna.ai>",
  "repository": {
    "type": "git",
    "url": "https://github.com/ajna-ai/ajna-sdk"
  },
  "engines": {
    "node": ">=18"
  },
  "publishConfig": {
    "access": "public"
  }
}
EOF

# Generate TypeScript declarations stub
cat > "${DIST_DIR}/ajna_verify.d.ts" <<EOF
/**
 * Ajna Verify WASM SDK
 * Post-quantum cryptographically signed identity verification.
 */

export interface AjnaConfig {
  apiKey: string;
  backendUrl: string;
  enablePqcSigning?: boolean;
}

export interface ScanResult {
  fields: DocumentField[];
  documentType: string;
  issuingCountry: string;
  confidence: number;
  pqcSignature: Uint8Array;
  pqcPublicKey: Uint8Array;
}

export interface DocumentField {
  key: string;
  value: string;
  confidence: number;
}

export interface QualityReport {
  gateReached: string;
  blurScore: number;
  meanLuma: number;
  motionScore: number;
  edgeDensity: number;
}

export declare function initAjnaVerify(config: AjnaConfig): Promise<void>;
export declare function processFrame(imageData: ImageData): Promise<QualityReport>;
export declare function getScanResult(): Promise<ScanResult | null>;
export declare function destroySession(): void;
EOF

# Generate README
cat > "${DIST_DIR}/README.md" <<EOF
# @ajna-ai/ajna-verify

Post-quantum cryptographically signed identity verification SDK for the web.

## Installation

\`\`\`bash
npm install @ajna-ai/ajna-verify
\`\`\`

## Quick Start

\`\`\`typescript
import { initAjnaVerify, processFrame, getScanResult } from '@ajna-ai/ajna-verify';

await initAjnaVerify({
  apiKey: 'your-api-key',
  backendUrl: 'https://api.ajna.ai',
});

// Process camera frames
const quality = await processFrame(imageData);
if (quality.gateReached === 'Accepted') {
  const result = await getScanResult();
  console.log(result?.fields);
}
\`\`\`

## Documentation

See the [Ajna Verify documentation](https://docs.ajna.ai/ajna-verify) for full API reference.
EOF

# Publish
echo "//registry.npmjs.org/:_authToken=\${NPM_TOKEN}" > "${DIST_DIR}/.npmrc"
cd "${DIST_DIR}" && npm publish --access public
echo "✓ Published @ajna-ai/ajna-verify@${VERSION}"
