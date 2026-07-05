// core/src/ffi.rs
// C-compatible FFI boundary.
// The C++ ML layer (tflite_bridge.cpp, coreml_bridge.mm) calls these functions
// to push inference results into the Rust session and retrieve the signed output.
// All pointers crossing this boundary are explicitly documented for lifetime.
//
// VR-4 (Security): Every exported function now returns i32.
//   0  = OK / success
//  -1  = null handle
//  -2  = null or invalid pointer argument
//  -3  = value out of range (e.g. field_count too large)
//  -4  = frame dimension validation failure
//
// README.md §VR-4 documents the rationale for status-code returns over panics.
// Panics across an FFI boundary are undefined behaviour in Rust.

use crate::frame::RawFrame;
use crate::quality::QualityGate;
use crate::result::{DocumentField, ScanResult};
use crate::session::{ScanSession, SessionConfig};
use base64::Engine;
use std::{boxed::Box, string::String, vec::Vec};

// ─── FFI status codes (matches ajna_ffi.h) ────────────────────────────────────
pub const AJNA_OK: i32 = 0;
pub const AJNA_ERR_NULL_HANDLE: i32 = -1;
pub const AJNA_ERR_NULL_PTR: i32 = -2;
pub const AJNA_ERR_OUT_OF_RANGE: i32 = -3;
pub const AJNA_ERR_INVALID_FRAME: i32 = -4;
pub const AJNA_ERR_INVALID_CONFIG: i32 = -5;

/// Maximum number of CField entries accepted per push_result call.
/// Prevents excessive heap allocation from a malicious bridge.
const MAX_FIELD_COUNT: usize = 256;
/// Maximum bytes for a single field key or value string.
const MAX_FIELD_STR_LEN: usize = 4096;

/// Opaque handle to a ScanSession. Returned to C++ as a raw pointer.
/// Caller MUST call ajna_session_destroy() when done.
pub type AjnaSessionHandle = *mut ScanSession;

/// Opaque handle to a QualityGate instance.
pub type AjnaGateHandle = *mut QualityGate;

// ─────────────────────────────────────────────────────────
// Session lifecycle
// ─────────────────────────────────────────────────────────

/// Create a new scan session with the given configuration.
/// Returns a non-null opaque handle on success, or null on allocation failure.
#[no_mangle]
pub extern "C" fn ajna_session_create(config: SessionConfig) -> AjnaSessionHandle {
    let session = Box::new(ScanSession::new(config));
    Box::into_raw(session)
}

/// Destroy a session handle previously obtained from ajna_session_create.
///
/// # Safety
/// `handle` must be a valid AjnaSessionHandle obtained from ajna_session_create,
/// or null. After this call, `handle` is invalid and must not be used.
/// Returns AJNA_OK (0). A null handle is a no-op (not an error).
#[no_mangle]
pub unsafe extern "C" fn ajna_session_destroy(handle: AjnaSessionHandle) -> i32 {
    if !handle.is_null() {
        // Safety: handle is a valid non-null pointer to a Box<ScanSession>
        drop(Box::from_raw(handle));
    }
    AJNA_OK
}

/// Transition the session to Scanning state and record the start timestamp.
///
/// # Safety
/// `handle` must be a valid, non-null AjnaSessionHandle.
/// Returns AJNA_ERR_NULL_HANDLE (-1) if handle is null.
#[no_mangle]
pub unsafe extern "C" fn ajna_session_start(handle: AjnaSessionHandle, timestamp_us: u64) -> i32 {
    if handle.is_null() {
        return AJNA_ERR_NULL_HANDLE;
    }
    let session = &mut *handle;
    session.start(timestamp_us);
    AJNA_OK
}

/// Returns the session state as a u32 (matches SessionState repr(C) discriminants).
/// 0=Idle, 1=Scanning, 2=Inferring, 3=Complete, 4=Failed.
/// Returns u32::MAX (0xFFFFFFFF) if handle is null.
///
/// # Safety
/// `handle` must be a valid, non-null AjnaSessionHandle.
#[no_mangle]
pub unsafe extern "C" fn ajna_session_get_state(handle: AjnaSessionHandle) -> u32 {
    if handle.is_null() {
        return u32::MAX;
    }
    let session = &*handle;
    session.state as u32
}

// ─────────────────────────────────────────────────────────
// Quality gate
// ─────────────────────────────────────────────────────────

/// Create a new quality gate with default thresholds.
#[no_mangle]
pub extern "C" fn ajna_gate_create() -> AjnaGateHandle {
    Box::into_raw(Box::new(QualityGate::default()))
}

/// Evaluate the quality gate on `frame`. Returns the Gate discriminant reached.
/// 0=Received, 1=BlurCheck, 2=ExposureCheck, 3=MotionCheck, 4=BoundaryCheck, 5=Accepted.
///
/// Returns u32::MAX if either pointer is null or if frame fails dimension validation
/// (VR-4: avoids UB when receiving a malformed frame from the C++ bridge).
///
/// # Safety
/// - `gate` must be a valid, non-null AjnaGateHandle.
/// - `frame` must be a valid pointer to a RawFrame. The frame's y_plane must be
///   valid for at least `frame.height * frame.y_stride` bytes.
#[no_mangle]
pub unsafe extern "C" fn ajna_gate_evaluate(gate: AjnaGateHandle, frame: *const RawFrame) -> u32 {
    if gate.is_null() || frame.is_null() {
        return u32::MAX;
    }
    let gate = &mut *gate;
    let frame_ref = &*frame;

    // VR-4: validate before any y_plane access inside evaluate().
    // evaluate() repeats this check internally; we mirror it here to catch it
    // as early as possible at the FFI layer.
    if frame_ref.validate().is_err() {
        // Gate::Received = 0; caller sees "frame not accepted" without UB.
        return 0;
    }

    let report = gate.evaluate(frame_ref);
    report.gate_reached as u32
}

/// Destroy a gate handle. A null handle is a no-op.
///
/// # Safety
/// `handle` must be a valid AjnaGateHandle, or null. Handle is invalid after this call.
#[no_mangle]
pub unsafe extern "C" fn ajna_gate_destroy(handle: AjnaGateHandle) -> i32 {
    if !handle.is_null() {
        // Safety: handle is a valid non-null pointer to a Box<QualityGate>
        drop(Box::from_raw(handle));
    }
    AJNA_OK
}

// ─────────────────────────────────────────────────────────
// Result ingestion (called by C++ ML layer after inference)
// ─────────────────────────────────────────────────────────

/// C-compatible field struct for crossing the FFI boundary.
/// All pointers must remain valid for the duration of the ajna_session_push_result call.
#[repr(C)]
pub struct CField {
    /// UTF-8 key bytes. NOT null-terminated; length given by key_len.
    pub key: *const u8,
    pub key_len: usize,
    /// UTF-8 value bytes. NOT null-terminated; length given by value_len.
    pub value: *const u8,
    pub value_len: usize,
    pub confidence: f32,
}

/// Called by the C++ inference layer to push the decoded result into the session.
/// Internally performs PQC signing if configured.
///
/// # Parameters
/// - `handle`          : valid, non-null AjnaSessionHandle
/// - `fields`          : pointer to at least `field_count` valid CField entries; may be null if field_count == 0
/// - `field_count`     : number of CField entries (0..=MAX_FIELD_COUNT)
/// - `doc_type_ptr`    : UTF-8 document type string bytes; length given by `doc_type_len`
/// - `country_ptr`     : UTF-8 issuing country bytes; length given by `country_len`
/// - `nonce_ptr`       : session nonce bytes (hex string); length given by `nonce_len`. May be null (len=0) when no backend binding required.
/// - `session_id_ptr`  : session UUID string bytes; length given by `session_id_len`. May be null (len=0).
/// - `overall_conf`    : overall confidence score 0.0–1.0
/// - `include_pqc_sig` : if true, sign with ML-DSA Level3 using the configured key strategy
///
/// # Return value
/// AJNA_OK (0) on success, negative on error (see FFI status codes at top of file).
///
/// # Safety
/// - `handle` must be a valid, non-null AjnaSessionHandle.
/// - `fields` must point to at least `field_count` valid CField entries (or be null when field_count == 0).
/// - All string pointers within each CField must be valid for their respective lengths.
/// - `doc_type_ptr` must be valid for `doc_type_len` bytes (or null if len == 0).
/// - `country_ptr` must be valid for `country_len` bytes (or null if len == 0).
/// - `nonce_ptr` and `session_id_ptr` may be null (treated as empty string).
/// - None of the above pointers need to remain valid after this function returns.
#[no_mangle]
pub unsafe extern "C" fn ajna_session_push_result(
    handle: AjnaSessionHandle,
    fields: *const CField,
    field_count: usize,
    doc_type_ptr: *const u8,
    doc_type_len: usize,
    country_ptr: *const u8,
    country_len: usize,
    nonce_ptr: *const u8,
    nonce_len: usize,
    session_id_ptr: *const u8,
    session_id_len: usize,
    overall_conf: f32,
    include_pqc_sig: bool,
) -> i32 {
    // VR-4: Null-check the session handle.
    if handle.is_null() {
        return AJNA_ERR_NULL_HANDLE;
    }

    // VR-4: Validate field_count before creating a slice.
    if field_count > MAX_FIELD_COUNT {
        return AJNA_ERR_OUT_OF_RANGE;
    }

    // VR-4: Null-check fields pointer when field_count > 0.
    if field_count > 0 && fields.is_null() {
        return AJNA_ERR_NULL_PTR;
    }

    // VR-4: Null-check required string pointers when their lengths are non-zero.
    if doc_type_len > 0 && doc_type_ptr.is_null() {
        return AJNA_ERR_NULL_PTR;
    }
    if country_len > 0 && country_ptr.is_null() {
        return AJNA_ERR_NULL_PTR;
    }

    // VR-4: Validate individual field string lengths.
    if field_count > 0 {
        let field_slice = core::slice::from_raw_parts(fields, field_count);
        for f in field_slice {
            if f.key_len > MAX_FIELD_STR_LEN || f.value_len > MAX_FIELD_STR_LEN {
                return AJNA_ERR_OUT_OF_RANGE;
            }
            if f.key_len > 0 && f.key.is_null() {
                return AJNA_ERR_NULL_PTR;
            }
            if f.value_len > 0 && f.value.is_null() {
                return AJNA_ERR_NULL_PTR;
            }
        }
    }

    let session = &mut *handle;

    let rust_fields: Vec<DocumentField> = if field_count > 0 {
        core::slice::from_raw_parts(fields, field_count)
            .iter()
            .map(|f| DocumentField {
                key: String::from_utf8_lossy(core::slice::from_raw_parts(f.key, f.key_len))
                    .into_owned(),
                value: String::from_utf8_lossy(core::slice::from_raw_parts(f.value, f.value_len))
                    .into_owned(),
                confidence: f.confidence,
            })
            .collect()
    } else {
        Vec::new()
    };

    let doc_type = if doc_type_len > 0 {
        String::from_utf8_lossy(core::slice::from_raw_parts(doc_type_ptr, doc_type_len))
            .into_owned()
    } else {
        String::new()
    };

    let country = if country_len > 0 {
        String::from_utf8_lossy(core::slice::from_raw_parts(country_ptr, country_len)).into_owned()
    } else {
        String::new()
    };

    // VR-1: Bind nonce and session_id into the ScanResult so they become part
    // of canonical_bytes() and are covered by the ML-DSA signature.
    let nonce = if nonce_len > 0 && !nonce_ptr.is_null() {
        Some(
            String::from_utf8_lossy(core::slice::from_raw_parts(nonce_ptr, nonce_len)).into_owned(),
        )
    } else {
        None
    };

    let session_id = if session_id_len > 0 && !session_id_ptr.is_null() {
        Some(
            String::from_utf8_lossy(core::slice::from_raw_parts(session_id_ptr, session_id_len))
                .into_owned(),
        )
    } else {
        None
    };

    // VR-1: Capture signing timestamp in UTC ISO-8601.
    let timestamp_iso = if nonce.is_some() || session_id.is_some() {
        // Use a simple Unix timestamp string when session binding is active.
        // The backend will validate this is within an acceptable freshness window.
        Some(get_unix_timestamp_str())
    } else {
        None
    };

    let mut result = ScanResult {
        fields: rust_fields,
        raw_mrz: None,
        document_type: doc_type,
        issuing_country: country,
        confidence: overall_conf,
        pqc_signature: Vec::new(),
        pqc_public_key: Vec::new(),
        nonce,
        session_id,
        timestamp_iso,
        algo: String::new(),
        ajna_version: String::from("2.0"),
        public_key: String::new(),
        jws_token: None,
    };

    if include_pqc_sig && session.config.pqc_sign_result {
        if let Ok(registry) = ajna_crypto::global_registry().read() {
            if let Some(signer) = registry.preferred() {
                let canonical = result.canonical_bytes();
                if let Ok(sig) = signer.sign(&canonical) {
                    result.pqc_signature = sig;
                    result.pqc_public_key = signer.public_key_bytes();
                    result.algo = signer.algorithm_id().to_string();
                    result.public_key =
                        base64::engine::general_purpose::STANDARD.encode(signer.public_key_bytes());
                    // Produce compact JWS for classical algorithms (ed25519, ecdsa-p256).
                    // Returns None for ml-dsa-65, hybrid, and on signing failure.
                    result.jws_token = ajna_crypto::jws::produce_jws(
                        &canonical,
                        signer.algorithm_id(),
                        signer.as_ref(),
                    );
                }
            }
        }
    }

    session.complete(result);
    AJNA_OK
}

/// Get current Unix timestamp as a decimal string (seconds since epoch).
/// Used for signing timestamp binding (VR-1). No_std compatible via libc.
fn get_unix_timestamp_str() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Safety: time() is always safe to call with a null pointer.
        let secs = unsafe { libc::time(core::ptr::null_mut()) };
        format!("{}", secs)
    }
    #[cfg(target_arch = "wasm32")]
    {
        // On WASM, timestamp is provided by the JS host and passed in via session_id binding.
        // Return a placeholder — the backend will validate via the nonce TTL instead.
        "0".to_string()
    }
}

// ─────────────────────────────────────────────────────────
// Result retrieval (called by platform layer to get JSON)
// ─────────────────────────────────────────────────────────

/// Serialize the completed session result to JSON into a caller-supplied buffer.
///
/// Return values:
///   positive       = bytes written (null terminator NOT counted)
///   -1             = null handle
///   -2             = session not in Complete state (state != 3)
///   -3             = internal serialization error
///   negative N where |N| > out_buf_len = buffer too small; retry with |N|+1 bytes
///
/// # Safety
/// handle must be a valid AjnaSessionHandle produced by ajna_session_create,
/// or null. out_buf must be valid for out_buf_len bytes when handle is non-null.
#[no_mangle]
pub unsafe extern "C" fn ajna_session_get_result_json(
    handle: AjnaSessionHandle,
    out_buf: *mut u8,
    out_buf_len: usize,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let session = &*handle;
    if session.state != crate::session::SessionState::Complete {
        return -2;
    }
    let result = match &session.result {
        Some(r) => r,
        None => return -2,
    };

    let json_val = serde_json::json!({
        "fields": result.fields.iter().map(|f| serde_json::json!({
            "key": f.key,
            "value": f.value,
            "confidence": f.confidence
        })).collect::<Vec<_>>(),
        "document_type": result.document_type,
        "issuing_country": result.issuing_country,
        "confidence": result.confidence,
        "pqc_signature_hex": hex::encode(&result.pqc_signature),
        "pqc_public_key_hex": hex::encode(&result.pqc_public_key),
        "algo": result.algo,
        "ajna_version": result.ajna_version,
        "public_key": result.public_key,
        "jws_token": result.jws_token
    });

    let json_bytes = match serde_json::to_vec(&json_val) {
        Ok(b) => b,
        Err(_) => return -3,
    };

    if json_bytes.len() >= out_buf_len {
        return -(json_bytes.len() as i32);
    }

    core::ptr::copy_nonoverlapping(json_bytes.as_ptr(), out_buf, json_bytes.len());
    out_buf.add(json_bytes.len()).write(0u8); // null terminator
    json_bytes.len() as i32
}

// ─── Declarative UI configuration validation ─────────────────────────────────

/// Maximum accepted UI config document size (bytes). A capture-UI theme has
/// no business being larger than this; the cap bounds FFI allocation.
const MAX_UI_CONFIG_LEN: usize = 64 * 1024;

/// Validate a declarative UI configuration JSON document (UTF-8, not
/// necessarily NUL-terminated). Platform shells call this once before
/// applying a client-supplied theme, so a malformed config is rejected at
/// the boundary instead of mid-capture.
///
/// Returns AJNA_OK (0) when the document parses and passes validation,
/// AJNA_ERR_NULL_PTR / AJNA_ERR_OUT_OF_RANGE for boundary violations, and
/// AJNA_ERR_INVALID_CONFIG (-5) for malformed JSON, bad colors, or values
/// outside product limits.
///
/// # Safety
///
/// `json_utf8` must be valid for `len` bytes for the duration of the call
/// only; the buffer may be freed immediately after return.
#[no_mangle]
pub unsafe extern "C" fn ajna_ui_config_validate(json_utf8: *const u8, len: usize) -> i32 {
    if json_utf8.is_null() {
        return AJNA_ERR_NULL_PTR;
    }
    if len == 0 || len > MAX_UI_CONFIG_LEN {
        return AJNA_ERR_OUT_OF_RANGE;
    }
    // Safety: FFI contract above — pointer valid for `len` bytes during this call.
    let bytes = unsafe { core::slice::from_raw_parts(json_utf8, len) };
    let Ok(json) = core::str::from_utf8(bytes) else {
        return AJNA_ERR_INVALID_CONFIG;
    };
    match crate::ui_config::UiConfig::from_json(json) {
        Ok(_) => AJNA_OK,
        Err(_) => AJNA_ERR_INVALID_CONFIG,
    }
}
