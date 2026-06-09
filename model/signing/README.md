# Model Signing

## Purpose

Beam Verify model artifacts are signed with Ed25519 to authenticate that Surt AI produced them. This is distinct from the ML-DSA (FIPS 204) result-level signature:

| Signature | Algorithm | Purpose |
|-----------|-----------|---------|
| Model signing | Ed25519 (classical) | Proves Surt produced the model file |
| Result signing | ML-DSA Level 3 (post-quantum) | Proves a scan result was produced by a genuine Beam SDK instance |

## Key Generation

Generate an Ed25519 signing key (keep this secret):

```bash
openssl genpkey -algorithm ed25519 -out model_signing_key.pem
```

Extract the public key (distribute this):

```bash
openssl pkey -in model_signing_key.pem -pubout -out model_signing_key_pub.pem
```

## Signing a Model

```bash
./sign_model.sh beam_idv_v1.tflite model_signing_key.pem
```

This produces `beam_idv_v1.tflite.sig` and prints the SHA256 hash.
Update `manifest/model_manifest.json` with the printed SHA256 value.

## Verifying a Model

```bash
./verify_model.sh beam_idv_v1.tflite model_signing_key_pub.pem
```

## Integration

The Beam SDK does not verify model signatures at runtime (this would require embedding the public key in the SDK binary). Model signature verification is:

1. **Build-time**: CI verifies the model signature before packaging into AAR/XCFramework/npm.
2. **Distribution-time**: The CDN or package registry verifies the signature before serving.
3. **Audit-time**: Security teams can verify model provenance at any point.
