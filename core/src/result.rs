// core/src/result.rs
// ScanResult and DocumentField — the output of the Ajna pipeline.
// canonical_bytes() produces a deterministic byte sequence suitable for PQC signing.
//
// VR-1 (Security): nonce, session_id, and timestamp_iso are now optional fields
// that, when present, are included in canonical_bytes() as reserved entries.
// This binds the signature to a specific session and prevents replay attacks.
// See README.md §VR-1 for full rationale.

use std::{string::String, vec::Vec};

/// A single extracted document field (e.g. "surname" = "SMITH").
#[derive(Debug, Clone)]
pub struct DocumentField {
    /// Field key (e.g. "surname", "given_names", "document_number").
    pub key: String,
    /// Decoded field value.
    pub value: String,
    /// Model confidence for this field, 0.0–1.0.
    pub confidence: f32,
}

/// The complete output of a Ajna scan session.
/// Produced by the Rust session after C++ inference pushes a result.
/// Optionally PQC-signed before delivery to the host application.
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// All extracted document fields, in unspecified order.
    /// canonical_bytes() sorts them deterministically.
    pub fields: Vec<DocumentField>,
    /// Raw MRZ string, if SessionConfig::include_raw_mrz is true.
    pub raw_mrz: Option<String>,
    /// ICAO document type string (e.g. "passport", "id_card", "residence_permit").
    pub document_type: String,
    /// ISO 3166-1 alpha-3 issuing country code (e.g. "USA", "GBR").
    pub issuing_country: String,
    /// Overall scan confidence, 0.0–1.0.
    pub confidence: f32,
    /// ML-DSA (Dilithium-3) signature over canonical_bytes(). Empty if pqc_sign_result=false.
    pub pqc_signature: Vec<u8>,
    /// Matching ML-DSA public key bytes. Empty if pqc_sign_result=false.
    pub pqc_public_key: Vec<u8>,

    // ── VR-1: Session-binding fields ────────────────────────────────────────
    // When these are Some, they are included in canonical_bytes() as reserved
    // entries (keys "__nonce", "__session_id", "__timestamp"), binding the
    // signature to a specific backend session and preventing replay.
    //
    // When None (SDK-only use, no backend), they are omitted from canonical_bytes()
    // for backward compatibility with callers that do not use the backend.
    /// Single-use nonce issued by the backend for this verification session.
    pub nonce: Option<String>,
    /// Backend session UUID string — ties this result to exactly one nonce.
    pub session_id: Option<String>,
    /// UTC signing timestamp (Unix seconds as decimal string).
    /// Backend validates this is within an acceptable freshness window.
    pub timestamp_iso: Option<String>,

    /// Signing algorithm identifier (e.g. "ed25519", "ml-dsa-65").
    /// Empty string when no signing was performed.
    pub algo: String,
    /// Ajna SDK version string. Always "2.0" for this release.
    pub ajna_version: String,
    /// Base64-encoded public key bytes from the signer.
    /// Empty string when no signing was performed.
    pub public_key: String,
    /// Compact JWS token (header.payload.signature) for classical algorithms only.
    /// Produced alongside pqc_signature for "ed25519" and "ecdsa-p256".
    /// Always None for "ml-dsa-65", "hybrid-*", and unsigned results.
    /// JWS signs the same canonical_bytes() as pqc_signature — it is an
    /// additional output format, NOT a replacement for the PQC signature.
    pub jws_token: Option<String>,
}

impl ScanResult {
    /// Produce a deterministic byte sequence for PQC signing.
    ///
    /// Encoding:
    ///   - Fields sorted lexicographically by key.
    ///   - Each field: 4-byte LE key length, key bytes, 4-byte LE value length, value bytes.
    ///   - NUL byte (0x00) delimiter between fields.
    ///   - document_type and issuing_country appended as final two fields with keys
    ///     "__document_type" and "__issuing_country" (double-underscore avoids collisions).
    ///
    /// VR-1 extension: when present, the following reserved fields are also included
    /// and sorted in-place with all other fields (they sort after "__issuing_country"
    /// and "__document_type" alphabetically):
    ///   "__nonce"      → nonce value
    ///   "__session_id" → session UUID string
    ///   "__timestamp"  → UTC signing timestamp string
    ///
    /// This encoding is deterministic regardless of field insertion order.
    /// Two ScanResults with identical field sets always produce identical canonical_bytes().
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut entries: Vec<(&str, &str)> = self
            .fields
            .iter()
            .map(|f| (f.key.as_str(), f.value.as_str()))
            .collect();

        // Add synthetic metadata fields with reserved key prefix.
        entries.push(("__document_type", self.document_type.as_str()));
        entries.push(("__issuing_country", self.issuing_country.as_str()));

        // VR-1: Include session-binding fields when present.
        if let Some(ref n) = self.nonce {
            entries.push(("__nonce", n.as_str()));
        }
        if let Some(ref s) = self.session_id {
            entries.push(("__session_id", s.as_str()));
        }
        if let Some(ref t) = self.timestamp_iso {
            entries.push(("__timestamp", t.as_str()));
        }

        // Sort by key — deterministic regardless of insertion order.
        entries.sort_by_key(|(k, _)| *k);

        let mut out = Vec::new();
        for (i, (key, value)) in entries.iter().enumerate() {
            // 4-byte LE key length
            let klen = key.len() as u32;
            out.extend_from_slice(&klen.to_le_bytes());
            // key bytes
            out.extend_from_slice(key.as_bytes());
            // 4-byte LE value length
            let vlen = value.len() as u32;
            out.extend_from_slice(&vlen.to_le_bytes());
            // value bytes
            out.extend_from_slice(value.as_bytes());
            // NUL delimiter between fields (not after the last)
            if i < entries.len() - 1 {
                out.push(0x00);
            }
        }
        out
    }
}
