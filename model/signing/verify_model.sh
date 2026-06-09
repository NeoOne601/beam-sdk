#!/usr/bin/env bash
# verify_model.sh — Verify a Beam model artifact's Ed25519 signature.
# Usage: ./verify_model.sh beam_idv_v1.tflite signing_key_pub.pem
# Reads: beam_idv_v1.tflite.sig (detached signature produced by sign_model.sh)
set -euo pipefail
MODEL="$1"
PUBKEY="$2"
SIG="${MODEL}.sig"
if [ ! -f "${MODEL}" ] || [ ! -f "${PUBKEY}" ] || [ ! -f "${SIG}" ]; then
    echo "Usage: $0 <model_file> <public_key.pem>"
    echo "Expects <model_file>.sig to exist (produced by sign_model.sh)"
    exit 1
fi
if openssl dgst -sha256 -verify "${PUBKEY}" -signature "${SIG}" "${MODEL}"; then
    echo "✓ Signature VALID for ${MODEL}"
    SHA256=$(sha256sum "${MODEL}" | awk '{print $1}')
    echo "SHA256: ${SHA256}"
    exit 0
else
    echo "✗ Signature INVALID for ${MODEL}"
    exit 1
fi
