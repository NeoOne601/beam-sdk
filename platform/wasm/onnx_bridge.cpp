// platform/wasm/onnx_bridge.cpp
// ONNX Runtime inference bridge for WebAssembly.
//
// MANDATORY COPY NOTE:
//   Unlike iOS (zero-copy CVPixelBuffer) and Android (zero-copy AHardwareBuffer),
//   WASM CANNOT achieve zero-copy from the browser's ImageData to the ONNX input tensor.
//   The copy from ImageData → OwnedFrame (Rust) → ONNX float tensor is an EXPECTED
//   and DOCUMENTED cost. At 25fps with 1920x1080 RGBA input this is ~5MB/s.
//   This is NOT a bug — document it clearly to all WASM-facing API consumers.
//
// Execution provider preference:
//   1. WebGPU EP (when available) — GPU-accelerated inference in browser
//   2. WASM SIMD CPU path (fallback) — always available with Emscripten SIMD flags
//
// Build requirement:
//   ONNX Runtime headers are available only during the Emscripten cross-compilation
//   build (ci/wasm_emscripten.sh). When building natively for IDE analysis or host
//   tests, the ORT headers are absent — the BEAM_HAS_ORT guard compiles a stub
//   implementation that logs a warning and returns early.

// ─── Detect ONNX Runtime availability ───────────────────────────────────────
//
// __has_include is standard C++17 (we require C++17 via CMakeLists.txt).
// When ORT headers are present (Emscripten build), BEAM_HAS_ORT is defined
// and the real implementation is compiled. Otherwise a stub path is used.
#if defined(__has_include)
  #if __has_include(<onnxruntime_cxx_api.h>)
    #define BEAM_HAS_ORT 1
  #endif
#endif

#ifdef BEAM_HAS_ORT
#include <onnxruntime_cxx_api.h>
#include <vector>
#include "../../include/beam_ffi.h"
#endif

#include <stdint.h>
#include <cstring>
#include <cstdio>

// ═══════════════════════════════════════════════════════════════════════════════
// REAL IMPLEMENTATION — compiled only when ONNX Runtime headers are available
// ═══════════════════════════════════════════════════════════════════════════════
#ifdef BEAM_HAS_ORT

// ─── Session type ────────────────────────────────────────────────────────────

struct BeamWasmSession {
    Ort::Env              env;
    Ort::Session          session;
    Ort::AllocatorWithDefaultOptions allocator;
    bool                  use_webgpu;

    BeamWasmSession(const char* model_path, bool use_webgpu_)
        : env(ORT_LOGGING_LEVEL_WARNING, "BeamWASM"),
          session(nullptr),
          use_webgpu(use_webgpu_)
    {
        Ort::SessionOptions opts;
        opts.SetIntraOpNumThreads(1); // Single thread on WASM — no threading overhead

        if (use_webgpu_) {
            // WebGPU Execution Provider — browser GPU-accelerated inference
            // Requires Emscripten with -s USE_WEBGPU=1 and ORT WASM EP build
            OrtStatus* status = OrtSessionOptionsAppendExecutionProvider_WebGPU(
                opts, nullptr);
            if (status) {
                fprintf(stderr, "[BeamWASM] WebGPU EP unavailable, falling back to WASM SIMD\n");
                Ort::GetApi().ReleaseStatus(status);
                use_webgpu_ = false;
            }
        }

        session = Ort::Session(env, model_path, opts);
    }
};

// ─── RGBA → normalised float tensor (mandatory copy on WASM) ─────────────────

/// Convert RGBA uint8 input to CHW float32 tensor normalised to [0, 1].
/// Layout: [1, 3, H, W] (batch=1, RGB channels, height, width).
///
/// NOTE: This copy is UNAVOIDABLE on WASM because ImageData lives in JS heap
/// memory that cannot be directly accessed by ONNX Runtime's C++ tensor allocator.
/// This is an expected cost — see module-level note.
static std::vector<float> rgba_to_chw_float(
    const uint8_t* rgba, int w, int h)
{
    std::vector<float> tensor(3 * h * w);
    for (int y = 0; y < h; ++y) {
        for (int x = 0; x < w; ++x) {
            int src = (y * w + x) * 4;
            int r_off = 0 * h * w + y * w + x;
            int g_off = 1 * h * w + y * w + x;
            int b_off = 2 * h * w + y * w + x;
            tensor[r_off] = rgba[src + 0] / 255.0f;
            tensor[g_off] = rgba[src + 1] / 255.0f;
            tensor[b_off] = rgba[src + 2] / 255.0f;
        }
    }
    return tensor;
}

// ─── C API (real) ────────────────────────────────────────────────────────────

extern "C" {

void* beam_wasm_create(const char* model_path) {
    try {
        bool try_webgpu = true; // Always attempt WebGPU first
        auto* s = new BeamWasmSession(model_path, try_webgpu);
        return s;
    } catch (const Ort::Exception& e) {
        fprintf(stderr, "[BeamWASM] Failed to create session: %s\n", e.what());
        return nullptr;
    }
}

void beam_wasm_process_frame(
    void*     ort_session,
    uint8_t*  rgba,
    int       w,
    int       h,
    void*     rust_session_handle)
{
    if (!ort_session || !rgba || !rust_session_handle) return;

    auto* s = reinterpret_cast<BeamWasmSession*>(ort_session);
    BeamSessionHandle rust_session =
        reinterpret_cast<BeamSessionHandle>(rust_session_handle);

    // Mandatory RGBA → CHW float tensor copy (documented expected cost)
    std::vector<float> input_tensor = rgba_to_chw_float(rgba, w, h);

    // Build ORT tensor from heap data
    std::vector<int64_t> input_shape = {1, 3, (int64_t)h, (int64_t)w};
    Ort::MemoryInfo mem_info =
        Ort::MemoryInfo::CreateCpu(OrtArenaAllocator, OrtMemTypeDefault);

    Ort::Value input_ort = Ort::Value::CreateTensor<float>(
        mem_info,
        input_tensor.data(), input_tensor.size(),
        input_shape.data(), input_shape.size());

    // Input/output names — must match the ONNX model's node names per output_schema.json
    const char* input_names[]  = { "image" };
    const char* output_names[] = { "confidence", "field_strings", "field_confidences" };

    try {
        auto outputs = s->session.Run(
            Ort::RunOptions{nullptr},
            input_names,  &input_ort, 1,
            output_names, 3);

        float confidence = 0.0f;
        if (!outputs.empty()) {
            auto* data = outputs[0].GetTensorData<float>();
            confidence = data ? data[0] : 0.0f;
        }

        // Parse field strings and confidences from output tensors
        static const char* field_keys[] = {
            "surname", "given_names", "date_of_birth", "document_number",
            "expiry_date", "mrz_line1", "mrz_line2", "sex", "nationality"
        };
        static const int N_FIELDS = 9;

        CField fields[N_FIELDS];
        int field_count = 0;

        // Output tensor 1: field_strings — packed null-terminated UTF-8 strings
        // Output tensor 2: field_confidences — float32 [N]
        if (outputs.size() >= 3) {
            auto str_info = outputs[1].GetTensorTypeAndShapeInfo();
            size_t str_total = str_info.GetElementCount();
            const char* str_data = reinterpret_cast<const char*>(outputs[1].GetTensorData<uint8_t>());

            const float* confs = outputs[2].GetTensorData<float>();
            auto conf_info = outputs[2].GetTensorTypeAndShapeInfo();
            int n_confs = static_cast<int>(conf_info.GetElementCount());

            size_t offset = 0;
            for (int i = 0; i < N_FIELDS && field_count < N_FIELDS && offset < str_total; ++i) {
                const char* field_val = str_data + offset;
                size_t field_len = strnlen(field_val, str_total - offset);

                if (field_len == 0) {
                    offset += 1;
                    continue;
                }

                fields[field_count].key        = reinterpret_cast<const uint8_t*>(field_keys[i]);
                fields[field_count].key_len    = strlen(field_keys[i]);
                fields[field_count].value      = reinterpret_cast<const uint8_t*>(field_val);
                fields[field_count].value_len  = field_len;
                fields[field_count].confidence = (i < n_confs) ? confs[i] : 0.5f;

                offset += field_len + 1;
                ++field_count;
            }
        }

        const char doc_type[] = "passport";
        const char country[]  = "UNK";
        beam_session_push_result(
            rust_session,
            fields, static_cast<size_t>(field_count),
            (const uint8_t*)doc_type, strlen(doc_type),
            (const uint8_t*)country,  strlen(country),
            /*nonce_ptr=*/      nullptr, /*nonce_len=*/      0,   // NEW VR-1 param 8-9
            /*session_id_ptr=*/ nullptr, /*session_id_len=*/ 0,   // NEW VR-1 param 10-11
            confidence,
            /*include_pqc_sig=*/ true
        );
    } catch (const Ort::Exception& e) {
        fprintf(stderr, "[BeamWASM] Inference error: %s\n", e.what());
    }
}

void beam_wasm_destroy(void* ort_session) {
    if (!ort_session) return;
    delete reinterpret_cast<BeamWasmSession*>(ort_session);
}

} // extern "C"

// ═══════════════════════════════════════════════════════════════════════════════
// STUB IMPLEMENTATION — compiled when ONNX Runtime headers are NOT available.
// This path exists so the file is valid C++ for IDE analysis, host-mode tests,
// and non-Emscripten builds. All functions log a warning and return early/null.
// ═══════════════════════════════════════════════════════════════════════════════
#else // !BEAM_HAS_ORT

extern "C" {

void* beam_wasm_create(const char* model_path) {
    (void)model_path;
    fprintf(stderr,
        "[BeamWASM] stub mode — ONNX Runtime not available.\n"
        "  This binary was compiled without <onnxruntime_cxx_api.h>.\n"
        "  Build with Emscripten + ORT headers for real inference.\n");
    return nullptr;
}

void beam_wasm_process_frame(
    void*     ort_session,
    uint8_t*  rgba,
    int       w,
    int       h,
    void*     rust_session_handle)
{
    (void)ort_session; (void)rgba; (void)w; (void)h; (void)rust_session_handle;
    fprintf(stderr, "[BeamWASM] stub mode — beam_wasm_process_frame is a no-op.\n");
}

void beam_wasm_destroy(void* ort_session) {
    (void)ort_session;
}

} // extern "C"

#endif // BEAM_HAS_ORT
