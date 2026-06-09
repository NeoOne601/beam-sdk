#!/usr/bin/env bash
# publish_npm.sh — Publish Beam Verify WASM SDK to npm.
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
  "name": "@surt-ai/beam-verify",
  "version": "${VERSION}",
  "description": "Post-quantum cryptographically signed identity verification SDK for the web",
  "main": "beam_verify.js",
  "types": "beam_verify.d.ts",
  "files": [
    "beam_verify.js",
    "beam_verify.wasm",
    "beam_verify.d.ts",
    "README.md"
  ],
  "keywords": ["identity-verification", "pqc", "wasm", "document-scanning", "ml-dsa"],
  "license": "SEE LICENSE IN LICENSE",
  "author": "Surt AI <sdk@surt.ai>",
  "repository": {
    "type": "git",
    "url": "https://github.com/surt-ai/beam-sdk"
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
cat > "${DIST_DIR}/beam_verify.d.ts" <<EOF
/**
 * Beam Verify WASM SDK
 * Post-quantum cryptographically signed identity verification.
 */

export interface BeamConfig {
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

export declare function initBeamVerify(config: BeamConfig): Promise<void>;
export declare function processFrame(imageData: ImageData): Promise<QualityReport>;
export declare function getScanResult(): Promise<ScanResult | null>;
export declare function destroySession(): void;
EOF

# Generate README
cat > "${DIST_DIR}/README.md" <<EOF
# @surt-ai/beam-verify

Post-quantum cryptographically signed identity verification SDK for the web.

## Installation

\`\`\`bash
npm install @surt-ai/beam-verify
\`\`\`

## Quick Start

\`\`\`typescript
import { initBeamVerify, processFrame, getScanResult } from '@surt-ai/beam-verify';

await initBeamVerify({
  apiKey: 'your-api-key',
  backendUrl: 'https://api.surt.ai',
});

// Process camera frames
const quality = await processFrame(imageData);
if (quality.gateReached === 'Accepted') {
  const result = await getScanResult();
  console.log(result?.fields);
}
\`\`\`

## Documentation

See the [Beam Verify documentation](https://docs.surt.ai/beam-verify) for full API reference.
EOF

# Publish
echo "//registry.npmjs.org/:_authToken=\${NPM_TOKEN}" > "${DIST_DIR}/.npmrc"
cd "${DIST_DIR}" && npm publish --access public
echo "✓ Published @surt-ai/beam-verify@${VERSION}"
