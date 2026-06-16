// core/fuzz/fuzz_targets/fuzz_push_result.rs
//! Fuzz target for beam_session_push_result FFI function.
//!
//! This fuzzes the primary attack surface: the CField array and string pointers
//! received from the C++ ML bridge. A malicious or buggy bridge could send:
//! - NULL key/value pointers
//! - key_len/value_len exceeding actual buffer size
//! - Non-UTF-8 byte sequences
//! - Extremely large field counts
//! - Empty strings, maximum-length strings

#![no_main]
use beam_core::ffi::{
    beam_session_create, beam_session_destroy, beam_session_push_result, beam_session_start, CField,
};
use beam_core::session::SessionConfig;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Need minimum data to construct meaningful inputs
    if data.len() < 16 {
        return;
    }

    // Create a session with default config (PQC signing disabled for speed)
    let config = SessionConfig {
        min_quality_frames: 3,
        timeout_ms: 30000,
        adaptive_gate_limit: 60,
        pqc_sign_result: false,
        include_raw_mrz: false,
    };

    let session = beam_session_create(config);
    if session.is_null() {
        return;
    }

    unsafe {
        beam_session_start(session, 1000);
    }

    // Parse fuzz data into field-like structures
    // First byte determines number of fields (capped to prevent OOM)
    let num_fields = (data[0] as usize).min(20);
    let mut fields: Vec<CField> = Vec::with_capacity(num_fields);
    let mut offset = 1;

    for _ in 0..num_fields {
        if offset + 4 >= data.len() {
            break;
        }

        // Derive lengths from fuzz data, capped to reasonable bounds
        let key_len = (data[offset] as usize)
            .min(64)
            .min(data.len().saturating_sub(offset + 3));
        let val_len = (data[offset + 1] as usize)
            .min(256)
            .min(data.len().saturating_sub(offset + 2 + key_len));

        let key_start = offset + 2;
        let val_start = key_start.saturating_add(key_len);

        if val_start.saturating_add(val_len) > data.len() {
            break;
        }

        // Construct CField with potentially invalid UTF-8 and edge-case lengths
        fields.push(CField {
            key: data[key_start..].as_ptr(),
            key_len,
            value: data[val_start..].as_ptr(),
            value_len: val_len,
            confidence: f32::from_bits(u32::from_le_bytes([
                data.get(offset + 2).copied().unwrap_or(0),
                data.get(offset + 3).copied().unwrap_or(0),
                data.get(offset + 4).copied().unwrap_or(0),
                data.get(offset + 5).copied().unwrap_or(0),
            ])),
        });

        offset = val_start.saturating_add(val_len);
    }

    // Use remaining data for doc_type and country strings
    let doc_type_ptr = data
        .get(offset)
        .map(|_| unsafe { data.as_ptr().add(offset) })
        .unwrap_or(std::ptr::null());
    let doc_type_len = data
        .get(offset)
        .map(|_| data.len().saturating_sub(offset).min(32))
        .unwrap_or(0);

    let country_offset = offset.saturating_add(doc_type_len);
    let country_ptr = data
        .get(country_offset)
        .map(|_| unsafe { data.as_ptr().add(country_offset) })
        .unwrap_or(std::ptr::null());
    let country_len = data
        .get(country_offset)
        .map(|_| data.len().saturating_sub(country_offset).min(16))
        .unwrap_or(0);

    unsafe {
        beam_session_push_result(
            session,
            fields.as_ptr(),
            fields.len(),
            doc_type_ptr,
            doc_type_len,
            country_ptr,
            country_len,
            std::ptr::null(), // nonce
            0,
            std::ptr::null(), // session_id
            0,
            0.5,   // overall_conf
            false, // include_pqc_sig - disabled for fuzzing speed
        );

        beam_session_destroy(session);
    }
});
