# Ajna Verify — Model Directory

This directory contains the model pack infrastructure for Ajna Verify.

## Structure

```
model/
├── manifest/
│   └── model_manifest.json    # Model pack manifest (3 platforms)
├── schema/
│   ├── output_schema.json     # Canonical model output schema
│   └── vocab_stub.json        # Placeholder vocab structure
├── signing/
│   ├── sign_model.sh          # Ed25519 signing script
│   ├── verify_model.sh        # Signature verification script
│   └── README.md              # Signing documentation
├── placeholder/
│   ├── PLACEHOLDER.tflite     # Empty TFLite stub
│   ├── PLACEHOLDER.mlpackage/ # Empty CoreML package
│   ├── PLACEHOLDER.onnx       # Empty ONNX stub
│   └── README.md              # Placeholder instructions
└── README.md                  # This file
```

## Workflow

1. **Schema first**: The model must conform to `schema/output_schema.json`.
2. **Convert**: Export the trained model to TFLite, CoreML, and ONNX formats.
3. **Replace**: Replace placeholder files with real model artifacts.
4. **Sign**: Use `signing/sign_model.sh` to create detached signatures.
5. **Manifest**: Update `manifest/model_manifest.json` with SHA256 hashes.
6. **CI**: The CI pipeline verifies signatures before packaging.

## Important Notes

- Model files are NOT committed to this repository (they are large binary artifacts).
- The placeholder files exist solely to document the expected file structure.
- Real model files are distributed via the Ajna model CDN.
- The `output_schema.json` is the single source of truth for the bridge ↔ model contract.
