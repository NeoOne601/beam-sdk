// core/src/ffi.rs
// C-compatible FFI boundary.
// The C++ ML layer (tflite_bridge.cpp, coreml_bridge.mm) calls these functions
// to push inference results into the Rust session and retrieve the signed output.
// All pointers crossing this boundary are explicitly documented for lifetime.

use crate::session::{ScanSession, SessionConfig};
use crate::result::{ScanResult, DocumentField};
use crate::quality::QualityGate;
use crate::frame::RawFrame;
use std::{boxed::Box, string::String, vec::Vec};

/// Opaque handle to a ScanSession. Returned to C++ as a raw pointer.
/// Caller MUST call beam_session_destroy() when done.
pub type BeamSessionHandle = *mut ScanSession;

/// Opaque handle to a QualityGate instance.
pub type BeamGateHandle = *mut QualityGate;

// ─────────────────────────────────────────────────────────
// Session lifecycle
// ─────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn beam_session_create(config: SessionConfig) -> BeamSessionHandle {
    let session = Box::new(ScanSession::new(config));
    Box::into_raw(session)
}

/// # Safety
/// `handle` must be a valid BeamSessionHandle obtained from beam_session_create,
/// or null. After this call, `handle` is invalid and must not be used.
#[no_mangle]
pub unsafe extern "C" fn beam_session_destroy(handle: BeamSessionHandle) {
    if !handle.is_null() {
        // Safety: handle is a valid non-null pointer to a Box<ScanSession>
        drop(Box::from_raw(handle));
    }
}

/// # Safety
/// `handle` must be a valid, non-null BeamSessionHandle.
#[no_mangle]
pub unsafe extern "C" fn beam_session_start(handle: BeamSessionHandle, timestamp_us: u64) {
    let session = &mut *handle;
    session.start(timestamp_us);
}

/// Returns the session state as a u32 (matches SessionState repr(C) discriminants).
/// 0=Idle, 1=Scanning, 2=Inferring, 3=Complete, 4=Failed
///
/// # Safety
/// `handle` must be a valid, non-null BeamSessionHandle.
#[no_mangle]
pub unsafe extern "C" fn beam_session_get_state(handle: BeamSessionHandle) -> u32 {
    let session = &*handle;
    session.state as u32
}

// ─────────────────────────────────────────────────────────
// Quality gate
// ─────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn beam_gate_create() -> BeamGateHandle {
    Box::into_raw(Box::new(QualityGate::default()))
}

/// Evaluate the quality gate on `frame`. Returns the Gate discriminant reached.
/// 0=Received, 1=BlurCheck, 2=ExposureCheck, 3=MotionCheck, 4=BoundaryCheck, 5=Accepted.
///
/// # Safety
/// - `gate` must be a valid, non-null BeamGateHandle.
/// - `frame` must be a valid pointer to a RawFrame whose y_plane is valid for
///   frame.width * frame.height bytes.
#[no_mangle]
pub unsafe extern "C" fn beam_gate_evaluate(
    gate:  BeamGateHandle,
    frame: *const RawFrame,
) -> u32 {
    let gate  = &mut *gate;
    let frame = &*frame;
    let report = gate.evaluate(frame);
    report.gate_reached as u32
}

/// # Safety
/// `handle` must be a valid BeamGateHandle, or null. Handle is invalid after this call.
#[no_mangle]
pub unsafe extern "C" fn beam_gate_destroy(handle: BeamGateHandle) {
    if !handle.is_null() {
        // Safety: handle is a valid non-null pointer to a Box<QualityGate>
        drop(Box::from_raw(handle));
    }
}

// ─────────────────────────────────────────────────────────
// Result ingestion (called by C++ ML layer after inference)
// ─────────────────────────────────────────────────────────

/// C-compatible field struct for crossing the FFI boundary.
/// All pointers must remain valid for the duration of the beam_session_push_result call.
#[repr(C)]
pub struct CField {
    /// UTF-8 key bytes. NOT null-terminated; length given by key_len.
    pub key:        *const u8,
    pub key_len:    usize,
    /// UTF-8 value bytes. NOT null-terminated; length given by value_len.
    pub value:      *const u8,
    pub value_len:  usize,
    pub confidence: f32,
}

/// Called by the C++ inference layer to push the decoded result into the session.
/// Internally performs PQC signing if configured.
///
/// # Safety
/// - `handle` must be a valid, non-null BeamSessionHandle.
/// - `fields` must point to at least `field_count` valid CField entries.
/// - All string pointers within each CField must be valid for their respective lengths.
/// - `doc_type_ptr` must be valid for `doc_type_len` bytes.
/// - `country_ptr` must be valid for `country_len` bytes.
/// - None of the above pointers need to remain valid after this function returns.
#[no_mangle]
pub unsafe extern "C" fn beam_session_push_result(
    handle:          BeamSessionHandle,
    fields:          *const CField,
    field_count:     usize,
    doc_type_ptr:    *const u8,
    doc_type_len:    usize,
    country_ptr:     *const u8,
    country_len:     usize,
    overall_conf:    f32,
    include_pqc_sig: bool,
) {
    let session = &mut *handle;

    let rust_fields: Vec<DocumentField> = core::slice::from_raw_parts(fields, field_count)
        .iter()
        .map(|f| DocumentField {
            key:        String::from_utf8_lossy(
                            core::slice::from_raw_parts(f.key, f.key_len)
                        ).into_owned(),
            value:      String::from_utf8_lossy(
                            core::slice::from_raw_parts(f.value, f.value_len)
                        ).into_owned(),
            confidence: f.confidence,
        })
        .collect();

    let doc_type = String::from_utf8_lossy(
        core::slice::from_raw_parts(doc_type_ptr, doc_type_len)
    ).into_owned();

    let country = String::from_utf8_lossy(
        core::slice::from_raw_parts(country_ptr, country_len)
    ).into_owned();

    let mut result = ScanResult {
        fields:          rust_fields,
        raw_mrz:         None,
        document_type:   doc_type,
        issuing_country: country,
        confidence:      overall_conf,
        pqc_signature:   Vec::new(),
        pqc_public_key:  Vec::new(),
    };

    if include_pqc_sig && session.config.pqc_sign_result {
        if let Ok(signer) = crate::crypto::PqcSigner::generate(
            crate::crypto::MlDsaLevel::Level3
        ) {
            let canonical = result.canonical_bytes();
            if let Ok(sig) = signer.sign(&canonical) {
                result.pqc_signature  = sig;
                result.pqc_public_key = signer.public_key_bytes().to_vec();
            }
        }
    }

    session.complete(result);
}
