#!/usr/bin/env bash
# scripts/package_wasm_npm.sh
# Package the Beam SDK WASM module as an npm package.
#
# Output: dist/BeamSDK-0.1.0.tgz
#
# Package structure (dist/npm/):
#   beam_sdk.wasm          — compiled WASM module
#   beam_sdk.js            — Emscripten JS loader (BeamModule factory)
#   package.json           — npm metadata
#   beam-sdk.d.ts          — TypeScript declarations
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="${REPO_ROOT}/dist"
WASM_SRC="${DIST_DIR}/wasm"
NPM_DIR="${DIST_DIR}/npm"
VERSION="0.1.0"

echo "=== Beam SDK WASM npm Packaging ==="
mkdir -p "${NPM_DIR}"

# ─── Build WASM (if not already built) ───────────────────────────────────────

echo "--- Building WASM module ---"
bash "${REPO_ROOT}/ci/wasm_emscripten.sh"

# ─── Copy WASM artifacts ─────────────────────────────────────────────────────

echo "--- Copying WASM artifacts ---"
cp "${WASM_SRC}/beam_sdk.wasm" "${NPM_DIR}/beam_sdk.wasm"
cp "${WASM_SRC}/beam_sdk.js"   "${NPM_DIR}/beam_sdk.js"

# ─── Write package.json ───────────────────────────────────────────────────────

echo "--- Writing package.json ---"
cat > "${NPM_DIR}/package.json" <<EOF
{
  "name": "@surt/beam-sdk",
  "version": "${VERSION}",
  "description": "Beam SDK — document scanning with PQC signing for Surt AI products",
  "main": "beam_sdk.js",
  "types": "beam-sdk.d.ts",
  "files": [
    "beam_sdk.js",
    "beam_sdk.wasm",
    "beam-sdk.d.ts"
  ],
  "keywords": ["beam", "sdk", "document-scanning", "pqc", "webassembly"],
  "author": "Surt AI",
  "license": "SEE LICENSE IN LICENSE",
  "engines": {
    "node": ">=18"
  },
  "repository": {
    "type": "git",
    "url": "https://github.com/surt-ai/beam-sdk"
  }
}
EOF

# ─── Write TypeScript declarations ───────────────────────────────────────────

echo "--- Writing TypeScript declarations ---"
cat > "${NPM_DIR}/beam-sdk.d.ts" <<'EOF'
/**
 * @surt/beam-sdk — TypeScript declarations
 *
 * MANDATORY COPY NOTE:
 *   WASM cannot access browser ImageData memory directly.
 *   You MUST copy ImageData.data into WASM heap before calling beam_wasm_process_frame().
 *   This is an expected cost, not a bug. See README for the recommended pattern.
 */

/** Opaque pointer to a native BeamWasmSession (ONNX Runtime). */
export type BeamSessionPtr = number;

/** Opaque pointer to a Rust ScanSession. */
export type RustSessionPtr = number;

/** Session state values returned by beam_session_get_state(). */
export const enum BeamSessionState {
    Idle      = 0,
    Scanning  = 1,
    Inferring = 2,
    Complete  = 3,
    Failed    = 4,
}

/** Configuration passed to beam_session_create() (matches Rust SessionConfig repr(C)). */
export interface BeamScanConfig {
    minQualityFrames:  number;  // default 3
    timeoutMs:         number;  // default 30000
    adaptiveGateLimit: number;  // default 60
    pqcSignResult:     boolean; // default true
    includeRawMrz:     boolean; // default false
}

/** A single extracted document field. */
export interface BeamDocumentField {
    key:        string;
    value:      string;
    confidence: number;
}

/** Scan result (populated when session state reaches Complete). */
export interface BeamScanResult {
    fields:         BeamDocumentField[];
    rawMrz?:        string;
    documentType:   string;
    issuingCountry: string;
    confidence:     number;
    pqcSignature:   Uint8Array;
    pqcPublicKey:   Uint8Array;
}

/**
 * The Emscripten module interface.
 * Access via: const Beam = await BeamModule();
 */
export interface BeamModule {
    // ─── WASM ONNX session lifecycle ──────────────────────────────────────────
    /** Load an ONNX model from the WASM virtual filesystem path. Returns 0 on failure. */
    beam_wasm_create(modelPath: string): BeamSessionPtr;
    /** Destroy a WASM ONNX session. */
    beam_wasm_destroy(session: BeamSessionPtr): void;

    /**
     * Process one RGBA frame.
     *
     * MANDATORY COPY: `rgbaPtr` must point to WASM heap memory containing
     * w*h*4 RGBA bytes. Copy from JS:
     *   const ptr = Module._malloc(w * h * 4);
     *   Module.HEAPU8.set(imageData.data, ptr);
     *   Module.beam_wasm_process_frame(wasmSession, ptr, w, h, rustSession);
     *   Module._free(ptr);
     */
    beam_wasm_process_frame(
        wasmSession:  BeamSessionPtr,
        rgbaPtr:      number,
        width:        number,
        height:       number,
        rustSession:  RustSessionPtr,
    ): void;

    // ─── Rust session lifecycle ───────────────────────────────────────────────
    beam_session_create(config: BeamScanConfig): RustSessionPtr;
    beam_session_destroy(session: RustSessionPtr): void;
    beam_session_start(session: RustSessionPtr, timestampUs: bigint): void;
    beam_session_get_state(session: RustSessionPtr): BeamSessionState;

    // ─── WASM memory utilities ────────────────────────────────────────────────
    _malloc(size: number): number;
    _free(ptr: number): void;
    HEAPU8: Uint8Array;
}

/** Factory function exported by the Emscripten loader. */
declare function BeamModule(options?: object): Promise<BeamModule>;
export default BeamModule;
EOF

# ─── Pack the npm package ────────────────────────────────────────────────────

echo "--- Running npm pack ---"
(cd "${DIST_DIR}" && npm pack npm/)

TARBALL="${DIST_DIR}/surt-beam-sdk-${VERSION}.tgz"
FINAL="${DIST_DIR}/BeamSDK-${VERSION}.tgz"

# Rename to canonical name
if [ -f "${TARBALL}" ]; then
    mv "${TARBALL}" "${FINAL}"
fi

echo ""
echo "=== npm packaging complete ==="
echo "  Output: ${FINAL}"
ls -lh "${FINAL}"
