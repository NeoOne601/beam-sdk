// tests/ffi_integration_tests.cpp
// C++ FFI integration test suite.
// Tests all #[no_mangle] exports from ffi.rs via the hand-written ajna_ffi.h header.
// Includes a timing assertion for gate evaluation performance (budget: < 4ms on A55).
//
// Build requirements:
//   - libajna_core.a must be in the link path
//   - ajna_ffi.h must be generated or hand-written (included below as an inline header)
//   - Compile: clang++ -O2 ffi_integration_tests.cpp -L../target/release -lajna_core -o ffi_tests
//
// Memory test:
//   valgrind --leak-check=full --error-exitcode=1 ./ffi_tests
//   Expected: 0 bytes definitely lost.

// Use the shared ajna_ffi.h header (canonical source of truth for FFI types).
#include "../include/ajna_ffi.h"

// ─── Test runner ─────────────────────────────────────────────────────────────

#include <cassert>
#include <cstdio>
#include <cstring>
#include <cstdlib>
#include <chrono>
#include <vector>

static int tests_run    = 0;
static int tests_passed = 0;

#define RUN_TEST(name) \
    do { \
        tests_run++; \
        if (test_##name()) { \
            tests_passed++; \
            printf("[PASS] %s\n", #name); \
        } else { \
            printf("[FAIL] %s\n", #name); \
        } \
    } while(0)

// ─── Default config helper ────────────────────────────────────────────────────

static AjnaSessionConfig default_config() {
    AjnaSessionConfig cfg;
    cfg.min_quality_frames  = 3;
    cfg.timeout_ms          = 30000;
    cfg.adaptive_gate_limit = 60;
    cfg.pqc_sign_result     = false; // disable PQC in FFI tests for speed
    cfg.include_raw_mrz     = false;
    return cfg;
}

// ─── Synthetic 1920×1080 Y plane (checkerboard for gate benchmark) ────────────

static std::vector<uint8_t> make_checkerboard_y(uint32_t w, uint32_t h) {
    std::vector<uint8_t> y(w * h);
    for (uint32_t row = 0; row < h; ++row) {
        for (uint32_t col = 0; col < w; ++col) {
            y[row * w + col] = ((row + col) % 2 == 0) ? 220 : 30;
        }
    }
    return y;
}

// ─── Tests ───────────────────────────────────────────────────────────────────

static bool test_session_create_returns_non_null() {
    AjnaSessionHandle h = ajna_session_create(default_config());
    bool ok = (h != nullptr);
    if (h) ajna_session_destroy(h);
    return ok;
}

static bool test_gate_create_returns_non_null() {
    AjnaGateHandle g = ajna_gate_create();
    bool ok = (g != nullptr);
    if (g) ajna_gate_destroy(g);
    return ok;
}

static bool test_session_start_transitions_to_scanning() {
    AjnaSessionHandle h = ajna_session_create(default_config());
    if (!h) return false;
    int32_t start_res = ajna_session_start(h, 0);
    uint32_t state = ajna_session_get_state(h);
    ajna_session_destroy(h);
    // State 1 = Scanning
    return (start_res == 0) && (state == 1);
}

static bool test_push_result_transitions_to_complete() {
    AjnaSessionHandle h = ajna_session_create(default_config());
    if (!h) return false;
    ajna_session_start(h, 0);

    // Push a result with two fields
    const char key1[]   = "surname";
    const char val1[]   = "SMITH";
    const char key2[]   = "given_names";
    const char val2[]   = "JOHN";
    const char dtype[]  = "passport";
    const char country[]= "USA";

    CField fields[2];
    fields[0].key        = (const uint8_t*)key1;
    fields[0].key_len    = strlen(key1);
    fields[0].value      = (const uint8_t*)val1;
    fields[0].value_len  = strlen(val1);
    fields[0].confidence = 0.99f;

    fields[1].key        = (const uint8_t*)key2;
    fields[1].key_len    = strlen(key2);
    fields[1].value      = (const uint8_t*)val2;
    fields[1].value_len  = strlen(val2);
    fields[1].confidence = 0.98f;

    int32_t push_res = ajna_session_push_result(
        h,
        fields, 2,
        (const uint8_t*)dtype,   strlen(dtype),
        (const uint8_t*)country, strlen(country),
        nullptr, 0, // nonce
        nullptr, 0, // session_id
        0.99f,
        false
    );

    uint32_t state = ajna_session_get_state(h);
    ajna_session_destroy(h);
    // State 3 = Complete
    return (push_res == 0) && (state == 3);
}

static bool test_session_destroy_no_crash() {
    AjnaSessionHandle h = ajna_session_create(default_config());
    int32_t res = ajna_session_destroy(h); // must not crash
    return res == 0;
}

static bool test_session_destroy_null_no_crash() {
    int32_t res = ajna_session_destroy(nullptr); // null guard must prevent crash
    return res == 0;
}

static bool test_gate_destroy_null_no_crash() {
    int32_t res = ajna_gate_destroy(nullptr); // null guard must prevent crash
    return res == 0;
}

// ─── Declarative UI config validation (ajna_ui_config_validate) ───────────────

static bool test_ui_config_valid_accepted() {
    // A minimal valid document ("{}" is the stock config) must return AJNA_OK.
    const char* json = "{\"mode\":\"custom\",\"theme\":{\"primary_color\":\"#FF5733\"}}";
    int32_t res = ajna_ui_config_validate(
        reinterpret_cast<const uint8_t*>(json), std::strlen(json));
    return res == 0; // AJNA_OK
}

static bool test_ui_config_malformed_rejected() {
    // A non-hex color must be rejected with AJNA_ERR_INVALID_CONFIG (-5).
    const char* json = "{\"theme\":{\"primary_color\":\"purple\"}}";
    int32_t res = ajna_ui_config_validate(
        reinterpret_cast<const uint8_t*>(json), std::strlen(json));
    return res == -5; // AJNA_ERR_INVALID_CONFIG
}

static bool test_ui_config_null_guarded() {
    // Null pointer must be guarded, not dereferenced.
    return ajna_ui_config_validate(nullptr, 16) == -2; // AJNA_ERR_NULL_PTR
}

// ─── Gate timing benchmark (must complete < 4ms on budget hardware) ───────────
// On a developer machine this will be much faster. The benchmark establishes
// that the gate code is O(n) and demonstrates the performance profile.
// On Cortex-A55 @ 2.0GHz the 4ms budget accommodates 1920x1080 with subsampling.

static bool test_gate_evaluate_timing_1080p() {
    const uint32_t W = 1920;
    const uint32_t H = 1080;
    auto y = make_checkerboard_y(W, H);
    std::vector<uint8_t> uv(W * H / 2, 128);

    AjnaRawFrame frame;
    frame.y_plane      = y.data();
    frame.uv_plane     = uv.data();
    frame.width        = W;
    frame.height       = H;
    frame.y_stride     = W;
    frame.uv_stride    = W;
    frame.format       = AJNA_FMT_NV12;
    frame.timestamp_us = 0;

    AjnaGateHandle gate = ajna_gate_create();
    if (!gate) return false;

    // Warm-up pass
    ajna_gate_evaluate(gate, &frame);

    // Timed pass
    auto t0 = std::chrono::high_resolution_clock::now();
    uint32_t result = ajna_gate_evaluate(gate, &frame);
    auto t1 = std::chrono::high_resolution_clock::now();

    ajna_gate_destroy(gate);

    double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
    printf("  [TIMING] gate_evaluate 1920x1080: %.3f ms (budget: < 4ms)\n", ms);

    (void)result;
    // On developer hardware this should be well under 1ms.
    // CI on ubuntu-latest may be slower — 20ms tolerance for CI correctness.
    // On Helio G85 the subsampled Sobel keeps this under 4ms.
    return (ms < 20.0); // CI-safe bound; real device bound is 4ms
}

// ─── Main ────────────────────────────────────────────────────────────────────

int main() {
    printf("=== Ajna SDK FFI Integration Tests ===\n\n");

    RUN_TEST(session_create_returns_non_null);
    RUN_TEST(gate_create_returns_non_null);
    RUN_TEST(session_start_transitions_to_scanning);
    RUN_TEST(push_result_transitions_to_complete);
    RUN_TEST(session_destroy_no_crash);
    RUN_TEST(session_destroy_null_no_crash);
    RUN_TEST(gate_destroy_null_no_crash);
    RUN_TEST(ui_config_valid_accepted);
    RUN_TEST(ui_config_malformed_rejected);
    RUN_TEST(ui_config_null_guarded);
    RUN_TEST(gate_evaluate_timing_1080p);

    printf("\n=== test result: %d/%d passed ===\n", tests_passed, tests_run);

    // MEMORY NOTE:
    // Run with: valgrind --leak-check=full --error-exitcode=1 ./ffi_tests
    // Expected output: "definitely lost: 0 bytes in 0 blocks"
    // The ajna_session_destroy and ajna_gate_destroy null guards ensure
    // no allocations are orphaned. Box::from_raw() in ffi.rs guarantees
    // the Rust allocator reclaims all heap memory on destroy.

    return (tests_passed == tests_run) ? 0 : 1;
}
