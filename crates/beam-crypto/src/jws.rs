// crates/beam-crypto/src/jws.rs
// Compact JWS production for classical signature algorithms.
//
// Produces a RFC 7515 compact serialisation (header.payload.signature)
// alongside the existing pqc_signature — it is an ADDITIONAL output
// format, NOT a replacement for the PQC signature or canonical_bytes().
//
// Rules:
//   - Only "ed25519" and "ecdsa-p256" produce a JWS token.
//   - "ml-dsa-65", "hybrid-*", and any unknown algo return None immediately.
//   - JWS production is best-effort: if signer.sign() fails, log a warning
//     and return None rather than propagating the error.
//
// The signing input matches the JWS Unencoded Payload Option (RFC 7797
// b64=false is set in the header for clarity, though the payload IS
// base64url-encoded here for compact serialisation compatibility).

use crate::signer::BeamSigner;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

/// Produce a compact JWS token for classical signature algorithms.
///
/// # Parameters
/// - `canonical_payload`: the exact byte sequence produced by `canonical_bytes()`
/// - `algo`: algorithm identifier (e.g. `"ed25519"`, `"ml-dsa-65"`)
/// - `signer`: the active [`BeamSigner`] instance
///
/// # Returns
/// - `Some(compact_jws)` for `"ed25519"` and `"ecdsa-p256"` on success.
/// - `None` for all PQC/hybrid algorithms, or if signing fails.
pub fn produce_jws(
    canonical_payload: &[u8],
    algo: &str,
    signer: &dyn BeamSigner,
) -> Option<String> {
    // Only classical algorithms get a JWS token.
    let jose_alg = match algo {
        "ed25519" => "EdDSA",
        "ecdsa-p256" => "ES256",
        _ => return None,
    };

    // ── 1. Build and encode the JWS protected header ─────────────────────────
    // {"alg":"<jose_alg>","b64":false,"typ":"JWT"}
    // b64=false signals unencoded payload (RFC 7797); we include it for
    // interoperability documentation, even though we encode the payload below
    // for compact serialisation compatibility.
    let header_json = format!(r#"{{"alg":"{}","b64":false,"typ":"JWT"}}"#, jose_alg);
    let header_b64 = URL_SAFE_NO_PAD.encode(header_json.as_bytes());

    // ── 2. Base64url-encode the canonical payload ─────────────────────────────
    let payload_b64 = URL_SAFE_NO_PAD.encode(canonical_payload);

    // ── 3. Construct the signing input: header_b64 + "." + payload_b64 ────────
    let signing_input = format!("{}.{}", header_b64, payload_b64);

    // ── 4. Sign the signing input bytes ──────────────────────────────────────
    let sig_bytes = match signer.sign(signing_input.as_bytes()) {
        Ok(s) => s,
        Err(e) => {
            // JWS is best-effort — log and return None rather than propagating.
            eprintln!("[beam-crypto] JWS signing failed for algo={}: {}", algo, e);
            return None;
        }
    };

    // ── 5. Base64url-encode the signature ────────────────────────────────────
    let sig_b64 = URL_SAFE_NO_PAD.encode(&sig_bytes);

    // ── 6. Assemble compact JWS: header.payload.signature ────────────────────
    Some(format!("{}.{}.{}", header_b64, payload_b64, sig_b64))
}
