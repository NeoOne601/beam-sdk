// Edge OCR: pixels → text → structured document fields → signed ScanResult.
//
// The pixel→text step is a pluggable native engine (Tesseract Mobile or
// PaddleOCR-Mobile on device); this crate owns the part that is portable and
// testable in pure Rust: turning the engine's raw text lines into typed,
// validated document fields for the three launch documents, and signing the
// result with ML-DSA-65 via the shared ajna-crypto registry.
//
// The native engine implements one trait (`OcrEngine`) — everything below is
// engine-agnostic and unit-tested against real document layouts.

use ajna_core::{DocumentField, ScanResult};
use ajna_crypto::{AjnaSigner, SignerError};
use base64::Engine as _;

/// Raw OCR output: the text lines a native engine read off one document image.
/// `mean_confidence` is the engine's own 0.0–1.0 confidence over the page.
#[derive(Debug, Clone)]
pub struct OcrText {
    pub lines: Vec<String>,
    pub mean_confidence: f32,
}

impl OcrText {
    pub fn new(lines: Vec<String>, mean_confidence: f32) -> Self {
        Self {
            lines,
            mean_confidence,
        }
    }
}

/// The one seam a native OCR engine fills. Given a grayscale/NV12 Y-plane of
/// `width`×`height`, return the recognized text lines. Implemented on-device by
/// the Tesseract/Paddle bridge; a fixture impl is used in tests.
pub trait OcrEngine {
    fn recognize(&self, y_plane: &[u8], width: u32, height: u32) -> Result<OcrText, OcrError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OcrError {
    EngineFailure(String),
    NoDocumentFound,
    UnrecognizedLayout,
}

impl std::fmt::Display for OcrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EngineFailure(m) => write!(f, "OCR engine failure: {m}"),
            Self::NoDocumentFound => write!(f, "no document found in frame"),
            Self::UnrecognizedLayout => write!(f, "document layout not recognized"),
        }
    }
}

impl std::error::Error for OcrError {}

/// The launch document types Ajna parses on the edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentKind {
    /// Indian Aadhaar card.
    Aadhaar,
    /// ICAO 9303 passport (TD3 machine-readable zone).
    Passport,
    /// US driver's license (AAMVA PDF417 text encoding).
    UsDriverLicense,
}

impl DocumentKind {
    /// ICAO-style document_type label used in `ScanResult`.
    fn document_type_label(self) -> &'static str {
        match self {
            Self::Aadhaar => "id_card",
            Self::Passport => "passport",
            Self::UsDriverLicense => "driving_license",
        }
    }
}

/// Parses raw OCR lines into a typed `ScanResult`. Detects the document kind
/// heuristically, then dispatches to the matching field extractor.
pub struct DocumentParser;

impl DocumentParser {
    /// Parse OCR text into a `ScanResult` (unsigned). Returns the detected kind
    /// alongside so callers can log/telemeter it.
    pub fn parse(ocr: &OcrText) -> Result<(DocumentKind, ScanResult), OcrError> {
        let kind = detect_kind(ocr).ok_or(OcrError::UnrecognizedLayout)?;
        let (fields, issuing_country, raw_mrz) = match kind {
            DocumentKind::Aadhaar => parse_aadhaar(ocr)?,
            DocumentKind::Passport => parse_passport_mrz(ocr)?,
            DocumentKind::UsDriverLicense => parse_us_dl(ocr)?,
        };
        if fields.is_empty() {
            return Err(OcrError::UnrecognizedLayout);
        }

        let result = ScanResult {
            fields,
            raw_mrz,
            document_type: kind.document_type_label().to_owned(),
            issuing_country,
            confidence: ocr.mean_confidence,
            ajna_version: "2.0".to_owned(),
            ..Default::default()
        };
        Ok((kind, result))
    }

    /// Full edge path: recognize with the native engine, parse, and sign the
    /// result with ML-DSA-65 (or any registry signer) so OCR output carries
    /// post-quantum provenance from the moment of capture.
    pub fn scan_and_sign(
        engine: &dyn OcrEngine,
        y_plane: &[u8],
        width: u32,
        height: u32,
        signer: &dyn AjnaSigner,
    ) -> Result<ScanResult, OcrError> {
        let ocr = engine.recognize(y_plane, width, height)?;
        let (_, mut result) = Self::parse(&ocr)?;
        sign_result(&mut result, signer)
            .map_err(|e| OcrError::EngineFailure(format!("signing failed: {e}")))?;
        Ok(result)
    }
}

/// Sign a `ScanResult` over its canonical bytes and populate the signature,
/// algorithm, and public-key fields. Shared by OCR and any other producer.
pub fn sign_result(result: &mut ScanResult, signer: &dyn AjnaSigner) -> Result<(), SignerError> {
    let canonical = result.canonical_bytes();
    let signature = signer.sign(&canonical)?;
    let public_key = signer.public_key_bytes();
    result.algo = signer.algorithm_id().to_owned();
    result.pqc_signature = signature;
    result.pqc_public_key = public_key.clone();
    result.public_key = base64::engine::general_purpose::STANDARD.encode(&public_key);
    Ok(())
}

fn field(key: &str, value: impl Into<String>, confidence: f32) -> DocumentField {
    DocumentField {
        key: key.to_owned(),
        value: value.into(),
        confidence,
    }
}

// ─── Layout detection ────────────────────────────────────────────────────────

fn detect_kind(ocr: &OcrText) -> Option<DocumentKind> {
    let joined = ocr.lines.join("\n");
    let upper = joined.to_ascii_uppercase();

    // Passport TD3: two 44-char MRZ lines, the first starting with 'P'.
    if ocr
        .lines
        .iter()
        .filter(|l| is_mrz_line(l) && l.len() >= 30)
        .count()
        >= 2
        && upper.contains('<')
    {
        return Some(DocumentKind::Passport);
    }
    // US DL: AAMVA element IDs (DAC given name, DCS surname, DAQ license number).
    if upper.contains("ANSI ") || (upper.contains("DAQ") && upper.contains("DCS")) {
        return Some(DocumentKind::UsDriverLicense);
    }
    // Aadhaar: 12-digit UID (often grouped 4-4-4) plus Aadhaar wording.
    if upper.contains("AADHAAR") || upper.contains("UNIQUE IDENTIFICATION") || has_aadhaar_uid(ocr)
    {
        return Some(DocumentKind::Aadhaar);
    }
    None
}

fn is_mrz_line(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '<')
}

fn has_aadhaar_uid(ocr: &OcrText) -> bool {
    ocr.lines.iter().any(|l| extract_aadhaar_uid(l).is_some())
}

// ─── Aadhaar ─────────────────────────────────────────────────────────────────

fn parse_aadhaar(ocr: &OcrText) -> Result<(Vec<DocumentField>, String, Option<String>), OcrError> {
    let conf = ocr.mean_confidence;
    let mut fields = Vec::new();

    if let Some(uid) = ocr.lines.iter().find_map(|l| extract_aadhaar_uid(l)) {
        // Verify Verhoeff checksum — Aadhaar's integrity check catches OCR digit slips.
        let digits: Vec<u8> = uid
            .bytes()
            .filter(|b| b.is_ascii_digit())
            .map(|b| b - b'0')
            .collect();
        let checksum_ok = verhoeff_valid(&digits);
        fields.push(field(
            "document_number",
            uid,
            if checksum_ok { conf } else { conf * 0.5 },
        ));
        fields.push(field(
            "aadhaar_checksum_valid",
            checksum_ok.to_string(),
            conf,
        ));
    }

    // Gender line (MALE / FEMALE) and DOB (DD/MM/YYYY) are stable Aadhaar tokens.
    for line in &ocr.lines {
        let upper = line.to_ascii_uppercase();
        if upper.contains("MALE") && !upper.contains("FEMALE") {
            fields.push(field("sex", "M", conf));
        } else if upper.contains("FEMALE") {
            fields.push(field("sex", "F", conf));
        }
        if let Some(dob) = extract_dob_ddmmyyyy(line) {
            fields.push(field("date_of_birth", dob, conf));
        }
    }
    Ok((fields, "IND".to_owned(), None))
}

/// Extract a 12-digit Aadhaar UID, tolerating the common 4-4-4 spacing.
fn extract_aadhaar_uid(line: &str) -> Option<String> {
    let digits: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
    // Reject lines that are mostly non-UID digits (e.g. a phone + year).
    if digits.len() == 12 && line.chars().filter(|c| c.is_alphabetic()).count() <= 2 {
        Some(digits)
    } else {
        None
    }
}

// ─── Passport (ICAO 9303 TD3 MRZ) ────────────────────────────────────────────

fn parse_passport_mrz(
    ocr: &OcrText,
) -> Result<(Vec<DocumentField>, String, Option<String>), OcrError> {
    let mrz: Vec<&String> = ocr
        .lines
        .iter()
        .filter(|l| is_mrz_line(l) && l.len() >= 30)
        .collect();
    if mrz.len() < 2 {
        return Err(OcrError::UnrecognizedLayout);
    }
    let l1 = normalize_mrz(mrz[0], 44);
    let l2 = normalize_mrz(mrz[1], 44);
    let conf = ocr.mean_confidence;
    let mut fields = Vec::new();

    // Line 1: P<ISSUER<SURNAME<<GIVEN<NAMES<<<...
    let issuing_country = l1.get(2..5).unwrap_or("").replace('<', "");
    if let Some(names) = l1.get(5..) {
        let mut parts = names.splitn(2, "<<");
        if let Some(surname) = parts.next() {
            fields.push(field("surname", mrz_text(surname), conf));
        }
        if let Some(given) = parts.next() {
            fields.push(field("given_names", mrz_text(given), conf));
        }
    }

    // Line 2: passport no (0..9) + check, nationality (10..13), DOB (13..19)+check,
    // sex (20), expiry (21..27)+check.
    if let Some(doc_no) = l2.get(0..9) {
        let doc_no_clean = doc_no.replace('<', "");
        let check_ok = l2
            .get(9..10)
            .and_then(|c| c.chars().next())
            .map(|c| mrz_check_digit(doc_no) == c)
            .unwrap_or(false);
        fields.push(field(
            "document_number",
            doc_no_clean,
            if check_ok { conf } else { conf * 0.5 },
        ));
        fields.push(field(
            "document_number_check_valid",
            check_ok.to_string(),
            conf,
        ));
    }
    if let Some(nat) = l2.get(10..13) {
        fields.push(field("nationality", nat.replace('<', ""), conf));
    }
    if let Some(dob) = l2.get(13..19) {
        fields.push(field("date_of_birth", mrz_date(dob), conf));
    }
    if let Some(sex) = l2.get(20..21) {
        fields.push(field("sex", sex.replace('<', "U"), conf));
    }
    if let Some(exp) = l2.get(21..27) {
        fields.push(field("expiry_date", mrz_date(exp), conf));
    }

    let raw_mrz = format!("{l1}\n{l2}");
    let country = if issuing_country.is_empty() {
        "UNK".to_owned()
    } else {
        issuing_country
    };
    Ok((fields, country, Some(raw_mrz)))
}

fn normalize_mrz(line: &str, len: usize) -> String {
    let mut s: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    while s.len() < len {
        s.push('<');
    }
    s.truncate(len);
    s
}

fn mrz_text(field: &str) -> String {
    field.trim_matches('<').replace('<', " ").trim().to_owned()
}

fn mrz_date(yymmdd: &str) -> String {
    // MRZ dates are YYMMDD; surface as-is (century resolution is a host concern).
    yymmdd.replace('<', "").to_owned()
}

/// ICAO 9303 check digit: weights cycle 7,3,1 over A=10..Z=35, '<'=0.
fn mrz_check_digit(field: &str) -> char {
    const WEIGHTS: [u32; 3] = [7, 3, 1];
    let sum: u32 = field
        .chars()
        .enumerate()
        .map(|(i, c)| mrz_char_value(c) * WEIGHTS[i % 3])
        .sum();
    char::from_digit(sum % 10, 10).unwrap_or('0')
}

fn mrz_char_value(c: char) -> u32 {
    match c {
        '0'..='9' => c as u32 - '0' as u32,
        'A'..='Z' => c as u32 - 'A' as u32 + 10,
        _ => 0, // '<' and fillers
    }
}

// ─── US Driver's License (AAMVA) ─────────────────────────────────────────────

fn parse_us_dl(ocr: &OcrText) -> Result<(Vec<DocumentField>, String, Option<String>), OcrError> {
    let conf = ocr.mean_confidence;
    let mut fields = Vec::new();

    // AAMVA element IDs are 3-letter prefixes on each line.
    for line in &ocr.lines {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("DAQ") {
            fields.push(field("document_number", v.trim(), conf));
        } else if let Some(v) = line.strip_prefix("DCS") {
            fields.push(field("surname", v.trim(), conf));
        } else if let Some(v) = line.strip_prefix("DAC") {
            fields.push(field("given_names", v.trim(), conf));
        } else if let Some(v) = line.strip_prefix("DBB") {
            fields.push(field("date_of_birth", aamva_date(v.trim()), conf));
        } else if let Some(v) = line.strip_prefix("DBA") {
            fields.push(field("expiry_date", aamva_date(v.trim()), conf));
        } else if let Some(v) = line.strip_prefix("DBC") {
            let sex = match v.trim() {
                "1" => "M",
                "2" => "F",
                _ => "U",
            };
            fields.push(field("sex", sex, conf));
        }
    }
    Ok((fields, "USA".to_owned(), None))
}

/// AAMVA dates are MMDDCCYY; normalize to CCYY-MM-DD when well-formed.
fn aamva_date(raw: &str) -> String {
    let d: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() == 8 {
        format!("{}-{}-{}", &d[4..8], &d[0..2], &d[2..4])
    } else {
        d
    }
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

/// Extract a DD/MM/YYYY (or DD-MM-YYYY) date, common on Aadhaar.
fn extract_dob_ddmmyyyy(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    let n = bytes.len();
    for i in 0..n.saturating_sub(9) {
        let w = &line[i..i + 10];
        let sep = w.as_bytes()[2];
        if (sep == b'/' || sep == b'-')
            && w.as_bytes()[5] == sep
            && w.bytes().enumerate().all(|(j, b)| {
                if j == 2 || j == 5 {
                    b == sep
                } else {
                    b.is_ascii_digit()
                }
            })
        {
            return Some(w.replace('-', "/"));
        }
    }
    None
}

/// Verhoeff checksum validation (the digit scheme Aadhaar uses). Returns true
/// when the trailing digit is a valid check over the preceding ones.
fn verhoeff_valid(digits: &[u8]) -> bool {
    const D: [[u8; 10]; 10] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
        [2, 3, 4, 0, 1, 7, 8, 9, 5, 6],
        [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
        [4, 0, 1, 2, 3, 9, 5, 6, 7, 8],
        [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
        [6, 5, 9, 8, 7, 1, 0, 4, 3, 2],
        [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
        [8, 7, 6, 5, 9, 3, 2, 1, 0, 4],
        [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
    ];
    const P: [[u8; 10]; 8] = [
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
        [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
        [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
        [9, 4, 5, 3, 1, 2, 6, 8, 7, 0],
        [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
        [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
        [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
    ];
    if digits.len() != 12 || digits.iter().any(|&d| d > 9) {
        return false;
    }
    let mut c = 0usize;
    for (i, &digit) in digits.iter().rev().enumerate() {
        c = D[c][P[i % 8][digit as usize] as usize] as usize;
    }
    c == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use ajna_crypto::MlDsaSigner;

    /// A canned OCR engine returning fixed lines — stands in for the native
    /// Tesseract/Paddle bridge so the parsing path is fully testable.
    struct FixtureEngine(OcrText);
    impl OcrEngine for FixtureEngine {
        fn recognize(&self, _y: &[u8], _w: u32, _h: u32) -> Result<OcrText, OcrError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn parses_passport_mrz_with_check_digit() {
        // Valid TD3 specimen (ICAO 9303 Appendix example).
        let l1 = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        let l2 = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";
        let ocr = OcrText::new(vec![l1.into(), l2.into()], 0.94);
        let (kind, result) = DocumentParser::parse(&ocr).expect("passport must parse");
        assert_eq!(kind, DocumentKind::Passport);
        assert_eq!(result.document_type, "passport");
        assert_eq!(result.issuing_country, "UTO");
        let surname = result.fields.iter().find(|f| f.key == "surname").unwrap();
        assert_eq!(surname.value, "ERIKSSON");
        let doc_no = result
            .fields
            .iter()
            .find(|f| f.key == "document_number")
            .unwrap();
        assert_eq!(doc_no.value, "L898902C3");
        // The specimen's check digit is valid, so confidence is not penalized.
        let check = result
            .fields
            .iter()
            .find(|f| f.key == "document_number_check_valid")
            .unwrap();
        assert_eq!(check.value, "true");
    }

    #[test]
    fn parses_us_driver_license_aamva() {
        let ocr = OcrText::new(
            vec![
                "ANSI 636000090002DL".into(),
                "DAQD12345678".into(),
                "DCSMORRISON".into(),
                "DACJOHN".into(),
                "DBB01151980".into(),
                "DBA01152028".into(),
                "DBC1".into(),
            ],
            0.9,
        );
        let (kind, result) = DocumentParser::parse(&ocr).expect("US DL must parse");
        assert_eq!(kind, DocumentKind::UsDriverLicense);
        assert_eq!(result.issuing_country, "USA");
        let dob = result
            .fields
            .iter()
            .find(|f| f.key == "date_of_birth")
            .unwrap();
        assert_eq!(dob.value, "1980-01-15");
        let sex = result.fields.iter().find(|f| f.key == "sex").unwrap();
        assert_eq!(sex.value, "M");
    }

    #[test]
    fn parses_aadhaar_with_verhoeff_check() {
        // 234123412346 is a canonical Verhoeff-valid Aadhaar test number.
        let ocr = OcrText::new(
            vec![
                "Government of India".into(),
                "Unique Identification Authority".into(),
                "2341 2341 2346".into(),
                "DOB: 01/01/1990".into(),
                "MALE".into(),
            ],
            0.88,
        );
        let (kind, result) = DocumentParser::parse(&ocr).expect("aadhaar must parse");
        assert_eq!(kind, DocumentKind::Aadhaar);
        assert_eq!(result.issuing_country, "IND");
        let uid = result
            .fields
            .iter()
            .find(|f| f.key == "document_number")
            .unwrap();
        assert_eq!(uid.value, "234123412346");
        let checksum = result
            .fields
            .iter()
            .find(|f| f.key == "aadhaar_checksum_valid")
            .unwrap();
        assert_eq!(checksum.value, "true");
        let dob = result
            .fields
            .iter()
            .find(|f| f.key == "date_of_birth")
            .unwrap();
        assert_eq!(dob.value, "01/01/1990");
    }

    #[test]
    fn aadhaar_bad_checksum_penalizes_confidence() {
        // Flip the last digit → Verhoeff must fail. UID sits on its own line,
        // as it does on a real card (the extractor rejects letter-heavy lines).
        let ocr = OcrText::new(vec!["Aadhaar".into(), "2341 2341 2347".into()], 0.9);
        let (_, result) = DocumentParser::parse(&ocr).expect("still parses");
        let checksum = result
            .fields
            .iter()
            .find(|f| f.key == "aadhaar_checksum_valid")
            .unwrap();
        assert_eq!(checksum.value, "false");
    }

    #[test]
    fn unrecognized_layout_is_an_error() {
        let ocr = OcrText::new(vec!["just some random text".into()], 0.5);
        assert_eq!(
            DocumentParser::parse(&ocr).unwrap_err(),
            OcrError::UnrecognizedLayout
        );
    }

    #[test]
    fn scan_and_sign_produces_ml_dsa_signature() {
        let l1 = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        let l2 = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";
        let engine = FixtureEngine(OcrText::new(vec![l1.into(), l2.into()], 0.94));
        let signer = MlDsaSigner::new();
        let result = DocumentParser::scan_and_sign(&engine, &[0u8; 64], 8, 8, &signer)
            .expect("edge scan+sign must succeed");
        assert_eq!(result.algo, "ml-dsa-65");
        assert!(!result.pqc_signature.is_empty());
        // The signature must verify against the produced canonical bytes.
        assert!(signer
            .verify(&result.canonical_bytes(), &result.pqc_signature)
            .expect("verify runs"));
    }
}
