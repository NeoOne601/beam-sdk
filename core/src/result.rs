// core/src/result.rs
// ScanResult and DocumentField — the output of the Beam pipeline.
// canonical_bytes() produces a deterministic byte sequence suitable for PQC signing.

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

/// The complete output of a Beam scan session.
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
    /// This encoding is deterministic regardless of field insertion order.
    /// Two ScanResults with identical field sets always produce identical canonical_bytes().
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut entries: Vec<(&str, &str)> = self
            .fields
            .iter()
            .map(|f| (f.key.as_str(), f.value.as_str()))
            .collect();

        // Add synthetic metadata fields with reserved key prefix
        let doc_type_key = "__document_type";
        let country_key = "__issuing_country";
        entries.push((doc_type_key, self.document_type.as_str()));
        entries.push((country_key, self.issuing_country.as_str()));

        // Sort by key — deterministic regardless of insertion order
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
