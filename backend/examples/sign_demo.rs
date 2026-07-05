// Demo signer: produces a genuinely valid /v1/verify request body.
//
// Reproduces the backend's `reconstruct_canonical_bytes` encoding exactly,
// signs it with a fresh Ed25519 key, and prints the JSON request. Used by the
// live end-to-end demo to drive a real PQC-signed verification through the
// running server (no camera/device — the document fields are a fixed passport
// specimen, signed for real).
//
//   cargo run --example sign_demo -- <nonce> <session_id> <timestamp_secs>

use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey};

fn encode_canonical(entries: &mut Vec<(String, String)>) -> Vec<u8> {
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::new();
    for (i, (k, v)) in entries.iter().enumerate() {
        out.extend_from_slice(&(k.len() as u32).to_le_bytes());
        out.extend_from_slice(k.as_bytes());
        out.extend_from_slice(&(v.len() as u32).to_le_bytes());
        out.extend_from_slice(v.as_bytes());
        if i < entries.len() - 1 {
            out.push(0x00);
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let nonce = args.get(1).cloned().unwrap_or_default();
    let session_id = args.get(2).cloned().unwrap_or_default();
    let timestamp = args.get(3).cloned().unwrap_or_default();

    // A fixed passport specimen (the field set an OCR pass would produce).
    let fields = [
        ("surname", "ERIKSSON"),
        ("given_names", "ANNA MARIA"),
        ("document_number", "L898902C3"),
    ];
    let document_type = "passport";
    let issuing_country = "UTO";

    // Mirror backend reconstruct_canonical_bytes: fields + __document_type +
    // __issuing_country + VR-1 __nonce/__session_id/__timestamp, sorted.
    let mut entries: Vec<(String, String)> =
        fields.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    entries.push(("__document_type".into(), document_type.into()));
    entries.push(("__issuing_country".into(), issuing_country.into()));
    entries.push(("__nonce".into(), nonce.clone()));
    entries.push(("__session_id".into(), session_id.clone()));
    entries.push(("__timestamp".into(), timestamp.clone()));
    let canonical = encode_canonical(&mut entries);

    let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
    let signature = signing_key.sign(&canonical);
    let b64 = base64::engine::general_purpose::STANDARD;

    let body = serde_json::json!({
        "session_id": session_id,
        "nonce": nonce,
        "scan_result": {
            "fields": fields.iter().map(|(k, v)| serde_json::json!({
                "key": k, "value": v, "confidence": 0.94
            })).collect::<Vec<_>>(),
            "document_type": document_type,
            "issuing_country": issuing_country,
            "confidence": 0.94,
            "pqc_signature": b64.encode(signature.to_bytes()),
            "key_id": session_id,           // tenant strategy ignores the value
            "signed_nonce": nonce,
            "signed_session_id": session_id,
            "signed_timestamp": timestamp,
            "algo": "ed25519",
            "public_key": b64.encode(signing_key.verifying_key().to_bytes()),
            "pqc_public_key": []
        }
    });
    println!("{body}");
}
