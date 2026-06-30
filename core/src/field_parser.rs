// core/src/field_parser.rs
// Bridge between raw model output (CField structs from C++ layer) and validated ScanResult.
// Validates field values against the canonical schema (model/schema/output_schema.json):
//   - Date format (YYYY-MM-DD)
//   - Country code length (ISO 3166-1 alpha-3)
//   - MRZ check digit validation per ICAO 9303 Part 3
//
// This module does NOT perform inference — it parses and validates the output
// of the C++ inference bridges (tflite_bridge, coreml_bridge, onnx_bridge).

use crate::ffi::CField;
use crate::result::{DocumentField, ScanResult};
use std::{string::String, vec::Vec};

/// Parsed and validated document extracted from model output.
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    /// Validated document fields.
    pub fields: Vec<DocumentField>,
    /// Document type string (e.g. "passport", "driving_licence").
    pub document_type: String,
    /// ISO 3166-1 alpha-3 issuing country code.
    pub issuing_country: String,
    /// Overall confidence score 0.0–1.0.
    pub confidence: f32,
    /// Fraud signal: probability this is a photo of a screen.
    pub is_screen_photo: f32,
    /// Fraud signal: probability this is a printed fake.
    pub is_printed_fake: f32,
    /// Whether the MRZ checksum validated successfully.
    pub mrz_checksum_valid: bool,
}

/// Error returned when field validation fails.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub reason: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseError: {}", self.reason)
    }
}

/// Stateless field parser. Validates raw CField data from the C++ inference layer.
pub struct FieldParser;

impl FieldParser {
    /// Parse and validate a slice of CField structs into a ParsedDocument.
    ///
    /// # Safety
    /// Each CField's key and value pointers must be valid for their respective lengths.
    /// This function copies all data out of the CField pointers immediately.
    pub unsafe fn parse(
        fields: &[CField],
        document_type: &str,
        issuing_country: &str,
        confidence: f32,
    ) -> Result<ParsedDocument, ParseError> {
        let mut doc_fields = Vec::with_capacity(fields.len());
        let mut mrz_line1: Option<String> = None;
        let mut mrz_line2: Option<String> = None;

        for field in fields {
            let key =
                String::from_utf8_lossy(core::slice::from_raw_parts(field.key, field.key_len))
                    .into_owned();
            let value =
                String::from_utf8_lossy(core::slice::from_raw_parts(field.value, field.value_len))
                    .into_owned();

            // Validate specific field types
            match key.as_str() {
                "date_of_birth" | "expiry_date" => {
                    if !value.is_empty() {
                        validate_date_format(&value)?;
                    }
                }
                "nationality" | "issuing_country" => {
                    if !value.is_empty() {
                        validate_country_code(&value)?;
                    }
                }
                "sex" => {
                    if !value.is_empty() && !matches!(value.as_str(), "M" | "F" | "X") {
                        return Err(ParseError {
                            reason: format!(
                                "Invalid sex value: '{}'. Expected M, F, X, or empty.",
                                value
                            ),
                        });
                    }
                }
                "mrz_line1" => {
                    mrz_line1 = Some(value.clone());
                }
                "mrz_line2" => {
                    mrz_line2 = Some(value.clone());
                }
                _ => {}
            }

            doc_fields.push(DocumentField {
                key,
                value,
                confidence: field.confidence,
            });
        }

        // Validate issuing country from the top-level field
        if !issuing_country.is_empty() && issuing_country.len() != 3 {
            // Allow 2-char country codes (some models output ISO alpha-2)
            // but flag anything else as invalid
            if issuing_country.len() != 2 {
                return Err(ParseError {
                    reason: format!(
                        "Invalid issuing_country length: {} (expected 2 or 3 chars, got '{}')",
                        issuing_country.len(),
                        issuing_country
                    ),
                });
            }
        }

        // MRZ checksum validation
        let mrz_checksum_valid = match (&mrz_line1, &mrz_line2) {
            (Some(l1), Some(l2)) => {
                validate_mrz_line1(l1).is_ok() && validate_mrz_line2(l2).is_ok()
            }
            _ => false, // No MRZ lines present — cannot validate
        };

        Ok(ParsedDocument {
            fields: doc_fields,
            document_type: document_type.to_string(),
            issuing_country: issuing_country.to_string(),
            confidence,
            is_screen_photo: 0.0,
            is_printed_fake: 0.0,
            mrz_checksum_valid,
        })
    }

    /// Convert a ParsedDocument into a ScanResult for the session.
    /// The VR-1 session-binding fields (nonce, session_id, timestamp_iso) are
    /// left as None here — they are populated by the FFI layer when a backend
    /// nonce is provided via beam_session_push_result().
    pub fn to_scan_result(doc: &ParsedDocument) -> ScanResult {
        ScanResult {
            fields: doc.fields.clone(),
            raw_mrz: None,
            document_type: doc.document_type.clone(),
            issuing_country: doc.issuing_country.clone(),
            confidence: doc.confidence,
            pqc_signature: Vec::new(),
            pqc_public_key: Vec::new(),
            nonce: None,
            session_id: None,
            timestamp_iso: None,
            algo: String::new(),
            beam_version: String::from("2.0"),
            public_key: String::new(),
            jws_token: None,
        }
    }
}

// ─── MRZ Check Digit (ICAO 9303 Part 3) ─────────────────────────────────────

/// Compute the ICAO 9303 check digit for a string.
///
/// Algorithm:
///   weights = [7, 3, 1] repeating
///   character values: 0-9 → 0-9, A-Z → 10-35, '<' → 0
///   check_digit = sum(char_value × weight) mod 10
pub fn mrz_check_digit(s: &str) -> u8 {
    let weights = [7u32, 3, 1];
    let sum: u32 = s
        .chars()
        .enumerate()
        .map(|(i, c)| {
            let val = mrz_char_value(c);
            val * weights[i % 3]
        })
        .sum();
    (sum % 10) as u8
}

/// Map an MRZ character to its numeric value per ICAO 9303.
fn mrz_char_value(c: char) -> u32 {
    match c {
        '0'..='9' => c as u32 - '0' as u32,
        'A'..='Z' => c as u32 - 'A' as u32 + 10,
        '<' => 0,
        _ => 0, // Invalid characters treated as filler
    }
}

/// Validate MRZ line 1 (TD3 passport format, 44 characters).
/// Format: P<COUNTRY_CODE<SURNAME<<GIVEN_NAMES<<<...
/// The line itself does not have a check digit, but we validate structure.
pub fn validate_mrz_line1(line: &str) -> Result<(), ParseError> {
    if line.is_empty() {
        return Ok(()); // Empty line is valid (no MRZ present)
    }
    // TD3 passport MRZ line 1 is 44 characters
    // TD1/TD2 cards have different lengths (30/36)
    let valid_lengths = [30, 36, 44];
    if !valid_lengths.contains(&line.len()) {
        return Err(ParseError {
            reason: format!(
                "MRZ line 1 invalid length: {} (expected 30, 36, or 44)",
                line.len()
            ),
        });
    }
    // Validate all characters are valid MRZ characters
    for c in line.chars() {
        if !matches!(c, '0'..='9' | 'A'..='Z' | '<') {
            return Err(ParseError {
                reason: format!("MRZ line 1 contains invalid character: '{}'", c),
            });
        }
    }
    Ok(())
}

/// Validate MRZ line 2 (TD3 passport format, 44 characters).
/// Contains check digits at specific positions that must validate.
///
/// TD3 Line 2 layout (44 chars):
///   [0..9]   Document number (9 chars)
///   [9]      Check digit over [0..9]
///   [10..12] Nationality (3 chars)
///   [13..18] Date of birth YYMMDD
///   [19]     Check digit over [13..19]
///   [20]     Sex (M/F/<)
///   [21..26] Expiry date YYMMDD
///   [27]     Check digit over [21..27]
///   [28..41] Optional data (14 chars)
///   [42]     Check digit over [28..42]
///   [43]     Composite check digit over [0..10]+[13..20]+[21..28]+[28..43]
pub fn validate_mrz_line2(line: &str) -> Result<(), ParseError> {
    if line.is_empty() {
        return Ok(());
    }
    let valid_lengths = [30, 36, 44];
    if !valid_lengths.contains(&line.len()) {
        return Err(ParseError {
            reason: format!(
                "MRZ line 2 invalid length: {} (expected 30, 36, or 44)",
                line.len()
            ),
        });
    }

    // Validate all characters
    for c in line.chars() {
        if !matches!(c, '0'..='9' | 'A'..='Z' | '<') {
            return Err(ParseError {
                reason: format!("MRZ line 2 contains invalid character: '{}'", c),
            });
        }
    }

    // TD3 (44 chars) check digit validation
    if line.len() == 44 {
        let chars: Vec<char> = line.chars().collect();

        // Check digit at position 9 over positions 0..9
        let doc_num: String = chars[0..9].iter().collect();
        let expected_cd_doc = mrz_check_digit(&doc_num);
        let actual_cd_doc = chars[9].to_digit(10).unwrap_or(99) as u8;
        if expected_cd_doc != actual_cd_doc {
            return Err(ParseError {
                reason: format!(
                    "MRZ document number check digit mismatch: expected {}, got {}",
                    expected_cd_doc, actual_cd_doc
                ),
            });
        }

        // Check digit at position 19 over positions 13..19
        let dob: String = chars[13..19].iter().collect();
        let expected_cd_dob = mrz_check_digit(&dob);
        let actual_cd_dob = chars[19].to_digit(10).unwrap_or(99) as u8;
        if expected_cd_dob != actual_cd_dob {
            return Err(ParseError {
                reason: format!(
                    "MRZ date of birth check digit mismatch: expected {}, got {}",
                    expected_cd_dob, actual_cd_dob
                ),
            });
        }

        // Check digit at position 27 over positions 21..27
        let expiry: String = chars[21..27].iter().collect();
        let expected_cd_exp = mrz_check_digit(&expiry);
        let actual_cd_exp = chars[27].to_digit(10).unwrap_or(99) as u8;
        if expected_cd_exp != actual_cd_exp {
            return Err(ParseError {
                reason: format!(
                    "MRZ expiry date check digit mismatch: expected {}, got {}",
                    expected_cd_exp, actual_cd_exp
                ),
            });
        }

        // Check digit at position 42 over positions 28..42
        let optional: String = chars[28..42].iter().collect();
        let expected_cd_opt = mrz_check_digit(&optional);
        let actual_cd_opt = chars[42].to_digit(10).unwrap_or(99) as u8;
        if expected_cd_opt != actual_cd_opt {
            return Err(ParseError {
                reason: format!(
                    "MRZ optional data check digit mismatch: expected {}, got {}",
                    expected_cd_opt, actual_cd_opt
                ),
            });
        }

        // Composite check digit at position 43 over [0..10]+[13..20]+[21..28]+[28..43]
        let composite_str: String = chars[0..10]
            .iter()
            .chain(chars[13..20].iter())
            .chain(chars[21..28].iter())
            .chain(chars[28..43].iter())
            .collect();
        let expected_cd_comp = mrz_check_digit(&composite_str);
        let actual_cd_comp = chars[43].to_digit(10).unwrap_or(99) as u8;
        if expected_cd_comp != actual_cd_comp {
            return Err(ParseError {
                reason: format!(
                    "MRZ composite check digit mismatch: expected {}, got {}",
                    expected_cd_comp, actual_cd_comp
                ),
            });
        }
    }

    Ok(())
}

// ─── Field Validation Helpers ────────────────────────────────────────────────

/// Validate YYYY-MM-DD date format.
fn validate_date_format(date: &str) -> Result<(), ParseError> {
    if date.len() != 10 {
        return Err(ParseError {
            reason: format!(
                "Invalid date length: {} (expected 10, YYYY-MM-DD)",
                date.len()
            ),
        });
    }
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err(ParseError {
            reason: format!("Invalid date format: '{}' (expected YYYY-MM-DD)", date),
        });
    }
    let year: u32 = parts[0].parse().map_err(|_| ParseError {
        reason: format!("Invalid year in date: '{}'", date),
    })?;
    let month: u32 = parts[1].parse().map_err(|_| ParseError {
        reason: format!("Invalid month in date: '{}'", date),
    })?;
    let day: u32 = parts[2].parse().map_err(|_| ParseError {
        reason: format!("Invalid day in date: '{}'", date),
    })?;

    if !(1900..=2100).contains(&year) {
        return Err(ParseError {
            reason: format!("Year out of range: {} (expected 1900-2100)", year),
        });
    }
    if !(1..=12).contains(&month) {
        return Err(ParseError {
            reason: format!("Month out of range: {} (expected 1-12)", month),
        });
    }
    if !(1..=31).contains(&day) {
        return Err(ParseError {
            reason: format!("Day out of range: {} (expected 1-31)", day),
        });
    }
    Ok(())
}

/// Validate ISO 3166-1 alpha-3 country code (3 uppercase letters).
fn validate_country_code(code: &str) -> Result<(), ParseError> {
    if code.len() != 3 {
        return Err(ParseError {
            reason: format!("Invalid country code length: {} (expected 3)", code.len()),
        });
    }
    if !code.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ParseError {
            reason: format!("Country code must be uppercase alpha-3: '{}'", code),
        });
    }
    Ok(())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mrz_char_values() {
        assert_eq!(mrz_char_value('0'), 0);
        assert_eq!(mrz_char_value('9'), 9);
        assert_eq!(mrz_char_value('A'), 10);
        assert_eq!(mrz_char_value('Z'), 35);
        assert_eq!(mrz_char_value('<'), 0);
    }

    #[test]
    fn test_mrz_check_digit_basic() {
        // Known test vector: "520727" → check digit 3
        // Weights: 5*7=35, 2*3=6, 0*1=0, 7*7=49, 2*3=6, 7*1=7 → sum=103 → 103%10=3
        assert_eq!(mrz_check_digit("520727"), 3);
    }

    #[test]
    fn test_mrz_check_digit_with_letters() {
        // "AB1234" → A=10, B=11, 1=1, 2=2, 3=3, 4=4
        // 10*7=70, 11*3=33, 1*1=1, 2*7=14, 3*3=9, 4*1=4 → sum=131 → 131%10=1
        assert_eq!(mrz_check_digit("AB1234"), 1);
    }

    #[test]
    fn test_mrz_check_digit_filler() {
        // "<<<" → all zeros
        // 0*7=0, 0*3=0, 0*1=0 → sum=0 → 0%10=0
        assert_eq!(mrz_check_digit("<<<"), 0);
    }

    #[test]
    fn test_mrz_check_digit_real_passport_doc_number() {
        // Common test: document number "L898902C3"
        // L=21, 8=8, 9=9, 8=8, 9=9, 0=0, 2=2, C=12, 3=3
        // 21*7=147, 8*3=24, 9*1=9, 8*7=56, 9*3=27, 0*1=0, 2*7=14, 12*3=36, 3*1=3
        // sum = 147+24+9+56+27+0+14+36+3 = 316 → 316%10 = 6
        assert_eq!(mrz_check_digit("L898902C3"), 6);
    }

    #[test]
    fn test_validate_date_valid() {
        assert!(validate_date_format("1990-05-15").is_ok());
        assert!(validate_date_format("2025-12-31").is_ok());
        assert!(validate_date_format("1900-01-01").is_ok());
    }

    #[test]
    fn test_validate_date_invalid() {
        assert!(validate_date_format("90-05-15").is_err());
        assert!(validate_date_format("1990/05/15").is_err());
        assert!(validate_date_format("1990-13-01").is_err());
        assert!(validate_date_format("1990-01-32").is_err());
        assert!(validate_date_format("").is_err());
    }

    #[test]
    fn test_validate_country_code_valid() {
        assert!(validate_country_code("USA").is_ok());
        assert!(validate_country_code("GBR").is_ok());
        assert!(validate_country_code("DEU").is_ok());
    }

    #[test]
    fn test_validate_country_code_invalid() {
        assert!(validate_country_code("US").is_err());
        assert!(validate_country_code("usa").is_err());
        assert!(validate_country_code("USAA").is_err());
        assert!(validate_country_code("U1A").is_err());
    }

    #[test]
    fn test_validate_mrz_line1_passport() {
        // Valid TD3 passport MRZ line 1 (44 chars)
        let line1 = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        assert!(validate_mrz_line1(line1).is_ok());
    }

    #[test]
    fn test_validate_mrz_line1_invalid_char() {
        let line1 = "P<UTO ERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<";
        assert!(validate_mrz_line1(line1).is_err());
    }

    #[test]
    fn test_validate_mrz_line1_invalid_length() {
        assert!(validate_mrz_line1("P<UTO").is_err());
    }

    #[test]
    fn test_validate_mrz_line2_passport() {
        // Valid ICAO 9303 TD3 Line 2 test vector:
        // L898902C36UTO7408122F1204159ZE184226B<<<<<10
        let line2 = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";
        // This is a real ICAO test vector — all check digits are correct
        assert!(validate_mrz_line2(line2).is_ok());
    }

    #[test]
    fn test_validate_mrz_line2_bad_check_digit() {
        // Corrupt the document number check digit (position 9: should be 6, set to 0)
        let line2 = "L898902C30UTO7408122F1204159ZE184226B<<<<<10";
        assert!(validate_mrz_line2(line2).is_err());
    }

    #[test]
    fn test_validate_mrz_line1_empty() {
        assert!(validate_mrz_line1("").is_ok());
    }

    #[test]
    fn test_validate_mrz_line2_empty() {
        assert!(validate_mrz_line2("").is_ok());
    }

    #[test]
    fn test_field_parser_basic() {
        // Create mock CField data
        let key = b"surname";
        let value = b"SMITH";
        let fields = [CField {
            key: key.as_ptr(),
            key_len: key.len(),
            value: value.as_ptr(),
            value_len: value.len(),
            confidence: 0.95,
        }];

        let result = unsafe { FieldParser::parse(&fields, "passport", "GBR", 0.92) };
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.fields.len(), 1);
        assert_eq!(doc.fields[0].key, "surname");
        assert_eq!(doc.fields[0].value, "SMITH");
        assert_eq!(doc.document_type, "passport");
        assert_eq!(doc.issuing_country, "GBR");
    }

    #[test]
    fn test_field_parser_invalid_date() {
        let key = b"date_of_birth";
        let value = b"not-a-date";
        let fields = [CField {
            key: key.as_ptr(),
            key_len: key.len(),
            value: value.as_ptr(),
            value_len: value.len(),
            confidence: 0.9,
        }];

        let result = unsafe { FieldParser::parse(&fields, "passport", "USA", 0.85) };
        assert!(result.is_err());
    }

    #[test]
    fn test_field_parser_invalid_sex() {
        let key = b"sex";
        let value = b"Z";
        let fields = [CField {
            key: key.as_ptr(),
            key_len: key.len(),
            value: value.as_ptr(),
            value_len: value.len(),
            confidence: 0.9,
        }];

        let result = unsafe { FieldParser::parse(&fields, "passport", "USA", 0.85) };
        assert!(result.is_err());
    }

    #[test]
    fn test_to_scan_result() {
        let doc = ParsedDocument {
            fields: vec![DocumentField {
                key: "surname".to_string(),
                value: "DOE".to_string(),
                confidence: 0.95,
            }],
            document_type: "passport".to_string(),
            issuing_country: "USA".to_string(),
            confidence: 0.92,
            is_screen_photo: 0.01,
            is_printed_fake: 0.02,
            mrz_checksum_valid: true,
        };
        let result = FieldParser::to_scan_result(&doc);
        assert_eq!(result.document_type, "passport");
        assert_eq!(result.fields.len(), 1);
        assert!(result.pqc_signature.is_empty());
    }
}
