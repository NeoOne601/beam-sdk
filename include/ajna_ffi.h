/*
 * ajna_ffi.h — Canonical C header for the Ajna SDK Rust FFI boundary.
 *
 * Generated from core/src/ffi.rs and core/src/session.rs.
 * In production, cbindgen should generate this file automatically.
 * This hand-written version is the source of truth until then.
 *
 * Consumers:
 *   platform/android/tflite_bridge.cpp
 *   platform/ios/coreml_bridge.mm
 *   platform/wasm/onnx_bridge.cpp
 *   tests/ffi_integration_tests.cpp
 */

#ifndef AJNA_FFI_H
#define AJNA_FFI_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ─── Opaque handles ──────────────────────────────────────────────────────── */

/** Opaque pointer to a Rust ScanSession. */
typedef void* AjnaSessionHandle;
/** Opaque pointer to a Rust QualityGate. */
typedef void* AjnaGateHandle;

/* ─── Session config (must match repr(C) layout in session.rs) ──────────── */

typedef struct {
    uint32_t min_quality_frames;
    uint64_t timeout_ms;
    uint32_t adaptive_gate_limit;
    bool     pqc_sign_result;
    bool     include_raw_mrz;
} AjnaSessionConfig;

/* ─── C-compatible field struct (mirrors CField in ffi.rs) ──────────────── */

typedef struct {
    const uint8_t* key;
    size_t         key_len;
    const uint8_t* value;
    size_t         value_len;
    float          confidence;
} CField;

/* ─── Pixel format (mirrors PixelFormat in frame.rs) ────────────────────── */

typedef enum {
    AJNA_FMT_NV12    = 0,
    AJNA_FMT_YUV420P = 1,
    AJNA_FMT_RGBA8   = 2,
} AjnaPixelFormat;

/* ─── Raw frame (must match repr(C) layout in frame.rs) ─────────────────── */

typedef struct {
    const uint8_t*  y_plane;
    const uint8_t*  uv_plane;
    uint32_t        width;
    uint32_t        height;
    uint32_t        y_stride;
    uint32_t        uv_stride;
    AjnaPixelFormat format;
    uint64_t        timestamp_us;
} AjnaRawFrame;

/* ─── Session lifecycle ─────────────────────────────────────────────────── */

AjnaSessionHandle ajna_session_create(AjnaSessionConfig config);
/** Null-safe. Returns 0 on success. */
int32_t           ajna_session_destroy(AjnaSessionHandle handle);
/** Returns 0 on success, negative on error. */
int32_t           ajna_session_start(AjnaSessionHandle handle, uint64_t timestamp_us);
/** Returns SessionState discriminant: 0=Idle, 1=Scanning, 2=Inferring, 3=Complete, 4=Failed. */
uint32_t          ajna_session_get_state(AjnaSessionHandle handle);

/* ─── Quality gate ──────────────────────────────────────────────────────── */

AjnaGateHandle ajna_gate_create(void);
/** Returns Gate discriminant: 0=Received, 1=BlurCheck, 2=ExposureCheck, 3=MotionCheck, 4=BoundaryCheck, 5=Accepted. */
uint32_t       ajna_gate_evaluate(AjnaGateHandle gate, const AjnaRawFrame* frame);
/** Null-safe. Returns 0 on success. */
int32_t        ajna_gate_destroy(AjnaGateHandle handle);

/* ─── Result ingestion (called by C++ ML bridge after inference) ────────── */

/**
 * Push the decoded inference result into the Rust session.
 *
 * @param handle         Valid, non-null AjnaSessionHandle.
 * @param fields         Pointer to field_count CField entries (may be NULL if field_count == 0).
 * @param field_count    Number of CField entries.
 * @param doc_type_ptr   UTF-8 document type string (not null-terminated).
 * @param doc_type_len   Length of doc_type_ptr in bytes.
 * @param country_ptr    UTF-8 country code string (not null-terminated).
 * @param country_len    Length of country_ptr in bytes.
 * @param nonce_ptr      UTF-8 session nonce (optional binding).
 * @param nonce_len      Length of nonce_ptr.
 * @param session_id_ptr UTF-8 session ID (optional binding).
 * @param session_id_len Length of session_id_ptr.
 * @param overall_conf   Overall confidence score, 0.0–1.0.
 * @param include_pqc_sig  If true AND session config has pqc_sign_result, signs the result.
 * @return 0 on success, negative on error.
 */
int32_t ajna_session_push_result(
    AjnaSessionHandle handle,
    const CField*     fields,
    size_t            field_count,
    const uint8_t*    doc_type_ptr,
    size_t            doc_type_len,
    const uint8_t*    country_ptr,
    size_t            country_len,
    const uint8_t*    nonce_ptr,
    size_t            nonce_len,
    const uint8_t*    session_id_ptr,
    size_t            session_id_len,
    float             overall_conf,
    bool              include_pqc_sig
);

/**
 * Serialize the completed session result to JSON into caller-supplied buffer.
 *
 * Return values:
 *   positive       = bytes written (null terminator NOT counted)
 *   -1             = null handle
 *   -2             = session not in Complete state (state != 3)
 *   -3             = internal serialization error
 *   negative N where |N| > out_buf_len = buffer too small; retry with |N|+1 bytes
 */
int32_t ajna_session_get_result_json(
    AjnaSessionHandle handle,
    uint8_t*          out_buf,
    size_t            out_buf_len
);

#ifdef __cplusplus
}
#endif

#endif /* AJNA_FFI_H */
