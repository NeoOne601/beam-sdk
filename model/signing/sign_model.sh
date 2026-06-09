#!/usr/bin/env bash
# sign_model.sh — Sign a Beam model artifact with an Ed25519 key.
# Usage: ./sign_model.sh beam_idv_v1.tflite signing_key.pem
# Produces: beam_idv_v1.tflite.sig (detached Ed25519 signature)
# NOTE: Model signing uses classical Ed25519 for the signing infrastructure itself.
# The Beam result-level signature uses ML-DSA (NIST FIPS 204). These serve
# different purposes: Ed25519 authenticates that Surt produced the model file;
# ML-DSA proves that a specific scan result was produced by a genuine Beam device.
set -euo pipefail
MODEL="$1"
KEY="$2"
if [ ! -f "${MODEL}" ] || [ ! -f "${KEY}" ]; then
    echo "Usage: $0 <model_file> <signing_key.pem>"
    exit 1
fi
openssl dgst -sha256 -sign "${KEY}" -out "${MODEL}.sig" "${MODEL}"
SHA256=$(sha256sum "${MODEL}" | awk '{print $1}')
echo "Signed: ${MODEL}"
echo "SHA256: ${SHA256}"
echo "Signature written to: ${MODEL}.sig"
echo "Update manifest/model_manifest.json sha256 field with: ${SHA256}"
