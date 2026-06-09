# Model Placeholders

The three files in this directory are **intentional empty artifacts** showing the required filename pattern for Beam Verify model files. They must be replaced with real model files before release.

## Required Files

| File | Platform | Format |
|------|----------|--------|
| `PLACEHOLDER.tflite` | Android | TensorFlow Lite FlatBuffer |
| `PLACEHOLDER.mlpackage/` | iOS | CoreML Package (directory) |
| `PLACEHOLDER.onnx` | WASM/Web | ONNX Runtime model |

## Replacement Instructions

1. Train or obtain a document recognition model following the schema in `../schema/output_schema.json`.
2. Convert to the three target formats (TFLite, CoreML, ONNX).
3. Replace the placeholder files with the real model files.
4. Run `sha256sum` on each model file and update `../manifest/model_manifest.json`.
5. Sign each model file with `../signing/sign_model.sh`.

## Input Tensor

- **Name**: `image`
- **Type**: float32
- **Shape**: [1, 3, H, W] (batch, channels, height, width)
- **Normalisation**: divide by 255
- **Colour space**: RGB (NV12 to RGB conversion performed by bridge)

## Output Tensors

Per `../schema/output_schema.json`:
- `confidence` — float32 [1], range [0.0, 1.0]
- `document_type` — int32 [1]
- `field_strings` — N null-terminated UTF-8 strings packed sequentially
- `field_confidences` — float32 [N] parallel to field_strings
- `fraud_signals` — float32 [3] (is_screen_photo, is_printed_fake, mrz_checksum_valid)

## Minimum Size Expectations

Real models are typically 5–50 MB. The placeholder files are 4 bytes each.
If your model file is under 1 MB, verify that it contains actual weights.

## Full Contract

See `../manifest/model_manifest.json` for the complete model pack specification.
