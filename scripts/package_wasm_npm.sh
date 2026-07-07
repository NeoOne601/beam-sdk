#!/usr/bin/env bash
# scripts/package_wasm_npm.sh
# Package the Ajna SDK WASM module as an npm package.
#
# Output: dist/AjnaSDK-0.1.0.tgz
#
# Package structure (dist/npm/):
#   ajna_sdk.wasm          — compiled WASM module
#   ajna_sdk.js            — Emscripten JS loader (AjnaModule factory)
#   package.json           — npm metadata
#   ajna-sdk.d.ts          — TypeScript declarations
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"
WASM_SRC="${DIST_DIR}/wasm"
NPM_DIR="${DIST_DIR}/npm"
VERSION="0.1.0"

echo "=== Ajna SDK WASM npm Packaging ==="
mkdir -p "${NPM_DIR}"

# ─── Build WASM (if not already built) ───────────────────────────────────────

echo "--- Building WASM module ---"
# bash "${REPO_ROOT}/ci/wasm_emscripten.sh"

# ─── Copy WASM artifacts ─────────────────────────────────────────────────────

echo "--- Copying WASM artifacts ---"
cp "${WASM_SRC}/ajna_sdk.wasm" "${NPM_DIR}/ajna_sdk.wasm"
cp "${WASM_SRC}/ajna_sdk.js"   "${NPM_DIR}/ajna_sdk.js"

# Copy TypeScript SDK wrapper
echo "--- Copying TypeScript SDK wrapper ---"
cp "${REPO_ROOT}/platform/web/AjnaScanner.ts" "${NPM_DIR}/AjnaScanner.ts"

# ─── Write package.json ───────────────────────────────────────────────────────

echo "--- Writing package.json ---"
cat > "${NPM_DIR}/package.json" <<EOF
{
  "name": "@ajna/sdk",
  "version": "${VERSION}",
  "description": "Ajna SDK — document scanning with PQC signing for Ajna AI products",
  "main": "AjnaScanner.js",
  "types": "ajna-sdk.d.ts",
  "files": [
    "ajna_sdk.js",
    "ajna_sdk.wasm",
    "ajna-sdk.d.ts",
    "AjnaScanner.ts",
    "AjnaScanner.js"
  ],
  "keywords": ["ajna", "sdk", "document-scanning", "pqc", "webassembly"],
  "author": "Ajna AI",
  "license": "SEE LICENSE IN LICENSE",
  "engines": {
    "node": ">=18"
  },
  "repository": {
    "type": "git",
    "url": "https://github.com/ajna-ai/ajna-sdk"
  }
}
EOF

# ─── Write TypeScript declarations ───────────────────────────────────────────

echo "--- Writing TypeScript declarations ---"
cat > "${NPM_DIR}/ajna-sdk.d.ts" <<'EOF'
/**
 * @ajna/sdk — TypeScript declarations
 *
 * MANDATORY COPY NOTE:
 *   WASM cannot access browser ImageData memory directly.
 *   You MUST copy ImageData.data into WASM heap before calling ajna_wasm_process_frame().
 *   This is an expected cost, not a bug. See README for the recommended pattern.
 */

/** Opaque pointer to a native AjnaWasmSession (ONNX Runtime). */
export type AjnaSessionPtr = number;

/** Opaque pointer to a Rust ScanSession. */
export type RustSessionPtr = number;

/** Session state values returned by ajna_session_get_state(). */
export const enum AjnaSessionState {
    Idle      = 0,
    Scanning  = 1,
    Inferring = 2,
    Complete  = 3,
    Failed    = 4,
}

/** Configuration passed to ajna_session_create() (matches Rust SessionConfig repr(C)). */
export interface AjnaScanConfig {
    minQualityFrames:  number;  // default 3
    timeoutMs:         number;  // default 30000
    adaptiveGateLimit: number;  // default 60
    pqcSignResult:     boolean; // default true
    includeRawMrz:     boolean; // default false
}

/** A single extracted document field. */
export interface AjnaDocumentField {
    key:        string;
    value:      string;
    confidence: number;
}

/** Scan result (populated when session state reaches Complete). */
export interface AjnaScanResult {
    fields:         AjnaDocumentField[];
    rawMrz?:        string;
    documentType:   string;
    issuingCountry: string;
    confidence:     number;
    pqcSignature:   Uint8Array;
    pqcPublicKey:   Uint8Array;
}

/**
 * The Emscripten module interface.
 * Access via: const Ajna = await AjnaModule();
 */
export interface AjnaModule {
    // ─── WASM ONNX session lifecycle ──────────────────────────────────────────
    /** Load an ONNX model from the WASM virtual filesystem path. Returns 0 on failure. */
    ajna_wasm_create(modelPath: string): AjnaSessionPtr;
    /** Destroy a WASM ONNX session. */
    ajna_wasm_destroy(session: AjnaSessionPtr): void;

    /**
     * Process one RGBA frame.
     *
     * MANDATORY COPY: `rgbaPtr` must point to WASM heap memory containing
     * w*h*4 RGBA bytes. Copy from JS:
     *   const ptr = Module._malloc(w * h * 4);
     *   Module.HEAPU8.set(imageData.data, ptr);
     *   Module.ajna_wasm_process_frame(wasmSession, ptr, w, h, rustSession);
     *   Module._free(ptr);
     */
    ajna_wasm_process_frame(
        wasmSession:  AjnaSessionPtr,
        rgbaPtr:      number,
        width:        number,
        height:       number,
        rustSession:  RustSessionPtr,
    ): void;

    // ─── Rust session lifecycle ───────────────────────────────────────────────
    ajna_session_create(config: AjnaScanConfig): RustSessionPtr;
    ajna_session_destroy(session: RustSessionPtr): void;
    ajna_session_start(session: RustSessionPtr, timestampUs: bigint): void;
    ajna_session_get_state(session: RustSessionPtr): AjnaSessionState;

    // ─── WASM memory utilities ────────────────────────────────────────────────
    _malloc(size: number): number;
    _free(ptr: number): void;
    HEAPU8: Uint8Array;
}

/** Factory function exported by the Emscripten loader. */
declare function AjnaModule(options?: object): Promise<AjnaModule>;
export default AjnaModule;
EOF

# ─── Pack the npm package ────────────────────────────────────────────────────

echo "--- Running npm pack ---"
(cd "${DIST_DIR}" && npm pack npm/)

TARBALL="${DIST_DIR}/ajna-sdk-${VERSION}.tgz"
FINAL="${DIST_DIR}/AjnaSDK-${VERSION}.tgz"

# Rename to canonical name
if [ -f "${TARBALL}" ]; then
    mv "${TARBALL}" "${FINAL}"
fi

echo ""
echo "=== npm packaging complete ==="
echo "  Output: ${FINAL}"
ls -lh "${FINAL}"

# TypeScript compilation (requires tsc on PATH — run manually or in CI after this script)
# tsc "${NPM_DIR}/AjnaScanner.ts" --outDir "${NPM_DIR}" --target ES2022 --module ESNext --strict
