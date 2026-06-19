#!/usr/bin/env python3
"""
scripts/generate_synthetic_model.py
Generate a synthetic TFLite model for end-to-end pipeline validation.

This model outputs CONSTANT placeholder values regardless of input pixels.
It is NOT a real document OCR model — it exists solely to exercise the full
pipeline: quality gates → inference → beam_session_push_result → Rust session
→ PQC signing → Complete state.

The model is INT8 quantized for TFLite GPU delegate compatibility.

MODEL ALWAYS OUTPUTS PLACEHOLDER VALUES:
  confidence       = ~0.87
  document_type    = ~0 (passport)
  field_strings    = ~"SMITH\0JOHN\0" (ASCII bytes via bias)
  field_confidences = ~[0.95, 0.93]

Execution environment: Apple M1 iMac with tensorflow-macos (ARM-native).
Usage: python3 scripts/generate_synthetic_model.py  (from repo root)
"""

import numpy as np
import tensorflow as tf
import os
import struct

# ─── Model Architecture ──────────────────────────────────────────────────────
#
# Input:  NHWC float32 [1, 224, 224, 3]
#   Note: The output_schema.json specifies NCHW [1, 3, H, W] for the canonical
#   model format. However, Keras defaults to NHWC and the C++ bridge handles
#   NV12 → RGB conversion with potential reordering. We use NHWC here since
#   keras defaults to it and it simplifies the synthetic model.
#
# Four output heads (all Dense layers with constant bias initialisers):
#   confidence       Dense(1)  → bias=0.87
#   document_type    Dense(1)  → bias=0.0  (int32 cast post-conversion)
#   field_confidences Dense(2) → bias=[0.95, 0.93]
#   field_strings    Dense(12) → bias=[83,77,73,84,72,0,74,79,72,78,0,0]
#                                   = ASCII "SMITH\0JOHN\0" + trailing 0

input_layer = tf.keras.Input(shape=(224, 224, 3), name="image")
pooled = tf.keras.layers.GlobalAveragePooling2D()(input_layer)

confidence_out = tf.keras.layers.Dense(
    1,
    name="confidence",
    kernel_initializer=tf.initializers.Constant(0.0),
    bias_initializer=tf.initializers.Constant(0.87),
)(pooled)

doc_type_out = tf.keras.layers.Dense(
    1,
    name="document_type",
    kernel_initializer=tf.initializers.Constant(0.0),
    bias_initializer=tf.initializers.Constant(0.0),
)(pooled)

field_conf_out = tf.keras.layers.Dense(
    2,
    name="field_confidences",
    kernel_initializer=tf.initializers.Constant(0.0),
    bias_initializer=tf.initializers.Constant([0.95, 0.93]),
)(pooled)

# field_strings: 12 units representing ASCII bytes for "SMITH\0JOHN\0\0"
field_str_out = tf.keras.layers.Dense(
    12,
    name="field_strings",
    kernel_initializer=tf.initializers.Constant(0.0),
    bias_initializer=tf.initializers.Constant(
        [83, 77, 73, 84, 72, 0, 74, 79, 72, 78, 0, 0]
    ),
)(pooled)

model = tf.keras.Model(
    inputs=input_layer,
    outputs=[confidence_out, doc_type_out, field_conf_out, field_str_out],
)

print("Model summary:")
model.summary()

# ─── TFLite Conversion (INT8 quantized) ──────────────────────────────────────

converter = tf.lite.TFLiteConverter.from_keras_model(model)
converter.optimizations = [tf.lite.Optimize.DEFAULT]


# Representative dataset required for INT8 quantization calibration
def representative_dataset():
    for _ in range(100):
        yield [np.random.uniform(0, 1, (1, 224, 224, 3)).astype(np.float32)]


converter.representative_dataset = representative_dataset

converter.target_spec.supported_ops = [
    tf.lite.OpsSet.TFLITE_BUILTINS_INT8
]
converter.inference_input_type = tf.float32   # keep float input for ease of use
converter.inference_output_type = tf.float32  # keep float output

tflite_model = converter.convert()

# ─── Output ──────────────────────────────────────────────────────────────────

OUTPUT_PATH = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "model", "placeholder", "synthetic_test.tflite"
)
os.makedirs(os.path.dirname(OUTPUT_PATH), exist_ok=True)

with open(OUTPUT_PATH, "wb") as f:
    f.write(tflite_model)

print(f"Synthetic model written to: {OUTPUT_PATH}")
print(f"Model size: {len(tflite_model) / 1024:.1f} KB")

# ─── Verification ────────────────────────────────────────────────────────────

interpreter = tf.lite.Interpreter(model_path=OUTPUT_PATH)
interpreter.allocate_tensors()
output_details = interpreter.get_output_details()
output_names = [d["name"] for d in output_details]
print(f"Output tensor names: {output_names}")
print("Synthetic model verification: PASS")
