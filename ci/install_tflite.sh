#!/usr/bin/env bash
# ci/install_tflite.sh
# Downloads and prepares TensorFlow Lite headers and Android prebuilt libraries.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TFLITE_DIR="${REPO_ROOT}/tflite"

echo "=== Installing TensorFlow Lite for Android ==="
mkdir -p "${TFLITE_DIR}/include" "${TFLITE_DIR}/lib/arm64-v8a" "${TFLITE_DIR}/lib/armeabi-v7a" "${TFLITE_DIR}/lib/x86_64"

# 1. Download and extract Maven AAR for .so files
echo "--- Downloading TFLite AAR ---"
AAR_URL="https://repo1.maven.org/maven2/org/tensorflow/tensorflow-lite/2.14.0/tensorflow-lite-2.14.0.aar"
curl -L "${AAR_URL}" -o "${TFLITE_DIR}/tflite.aar"
unzip -o -d "${TFLITE_DIR}/aar_extract" "${TFLITE_DIR}/tflite.aar"

echo "--- Copying prebuilt .so libraries ---"
cp "${TFLITE_DIR}/aar_extract/jni/arm64-v8a/libtensorflowlite_jni.so"   "${TFLITE_DIR}/lib/arm64-v8a/libtensorflowlite.so"
cp "${TFLITE_DIR}/aar_extract/jni/armeabi-v7a/libtensorflowlite_jni.so" "${TFLITE_DIR}/lib/armeabi-v7a/libtensorflowlite.so"
cp "${TFLITE_DIR}/aar_extract/jni/x86_64/libtensorflowlite_jni.so"      "${TFLITE_DIR}/lib/x86_64/libtensorflowlite.so"

# 2. Download and extract TensorFlow C++ headers (v2.14.0)
echo "--- Downloading TensorFlow source for headers ---"
TF_ZIP_URL="https://github.com/tensorflow/tensorflow/archive/refs/tags/v2.14.0.zip"
curl -L "${TF_ZIP_URL}" -o "${TFLITE_DIR}/tf.zip"
unzip -o -q -d "${TFLITE_DIR}/tf_extract" "${TFLITE_DIR}/tf.zip"
mv "${TFLITE_DIR}/tf_extract/tensorflow-2.14.0/tensorflow" "${TFLITE_DIR}/include/"

# 3. Download and extract FlatBuffers headers (v23.5.26)
echo "--- Downloading FlatBuffers headers ---"
FB_ZIP_URL="https://github.com/google/flatbuffers/archive/refs/tags/v23.5.26.zip"
curl -L "${FB_ZIP_URL}" -o "${TFLITE_DIR}/fb.zip"
unzip -o -q -d "${TFLITE_DIR}/fb_extract" "${TFLITE_DIR}/fb.zip"
mv "${TFLITE_DIR}/fb_extract/flatbuffers-23.5.26/include/flatbuffers" "${TFLITE_DIR}/include/"

# 4. Download and extract Abseil headers (20230802.1)
echo "--- Downloading Abseil headers ---"
ABS_ZIP_URL="https://github.com/abseil/abseil-cpp/archive/refs/tags/20230802.1.zip"
curl -L "${ABS_ZIP_URL}" -o "${TFLITE_DIR}/abs.zip"
unzip -o -q -d "${TFLITE_DIR}/abs_extract" "${TFLITE_DIR}/abs.zip"
mv "${TFLITE_DIR}/abs_extract/abseil-cpp-20230802.1/absl" "${TFLITE_DIR}/include/"

# Clean up temporary downloads
echo "--- Cleaning up temporary files ---"
rm -rf "${TFLITE_DIR}/aar_extract" "${TFLITE_DIR}/tflite.aar"
rm -rf "${TFLITE_DIR}/tf_extract" "${TFLITE_DIR}/tf.zip"
rm -rf "${TFLITE_DIR}/fb_extract" "${TFLITE_DIR}/fb.zip"
rm -rf "${TFLITE_DIR}/abs_extract" "${TFLITE_DIR}/abs.zip"

echo "=== TFLite installation complete ==="
