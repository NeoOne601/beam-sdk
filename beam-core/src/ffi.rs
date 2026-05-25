// beam-core/src/ffi.rs
// C-compatible FFI boundary.
// The C++ ML layer (tflite_bridge.cpp, coreml_bridge.mm) calls these functions
// to push inference results into the Rust session and retrieve the signed output.
// All pointers crossing this boundary are explicitly documented for lifetime.

use crate::session::{ScanSession, SessionConfig};
use crate::result::{ScanResult, DocumentField};
use crate::quality::QualityGate;
use crate::frame::RawFrame;
use alloc::{boxed::Box, string::String, vec::Vec};

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

/// # Safety: handle must be a valid BeamSessionHandle obtained from beam_session_create.
#[no_mangle]
pub unsafe extern "C" fn beam_session_destroy(handle: BeamSessionHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// # Safety: handle must be valid.
#[no_mangle]
pub unsafe extern "C" fn beam_session_start(handle: BeamSessionHandle, timestamp_us: u64) {
    let session = &mut *handle;
    session.start(timestamp_us);
}

// ─────────────────────────────────────────────────────────
// Quality gate
// ─────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn beam_gate_create() -> BeamGateHandle {
    Box::into_raw(Box::new(QualityGate::default()))
}

/// # Safety: handle must be valid, frame pointer must be valid for frame dimensions.
#[no_mangle]
pub unsafe extern "C" fn beam_gate_evaluate(
    gate:   BeamGateHandle,
    frame:  *const RawFrame,
) -> u32 /* Gate discriminant */ {
    let gate  = &mut *gate;
    let frame = &*frame;
    let report = gate.evaluate(frame);
    report.gate_reached as u32
}

#[no_mangle]
pub unsafe extern "C" fn beam_gate_destroy(handle: BeamGateHandle) {
    if !handle.is_null() { drop(Box::from_raw(handle)); }
}

// ─────────────────────────────────────────────────────────
// Result ingestion (called by C++ ML layer after inference)
// ─────────────────────────────────────────────────────────

/// C-compatible field struct for crossing the FFI boundary.
#[repr(C)]
pub struct CField {
    /// Null-terminated UTF-8 key.
    pub key:        *const u8,
    pub key_len:    usize,
    /// Null-terminated UTF-8 value.
    pub value:      *const u8,
    pub value_len:  usize,
    pub confidence: f32,
}

/// Called by the C++ inference layer to push the decoded result into the session.
///
/// # Safety
/// - `handle` must be a valid session.
/// - `fields` must point to `field_count` valid CField entries.
/// - All string pointers within CField must be valid for their respective lengths.
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
