// backend/src/crypto/ed25519_verifier.rs
// Ed25519 signature verification for the backend.
//
// Pure verification only — no signing, no key generation.
// Uses ed25519-dalek (already present in Cargo.toml).

use ed25519_dalek::{Signature, VerifyingKey};

/// Errors that can occur during Ed25519 signature verification.
#[derive(Debug)]
pub enum Ed25519VerifyError {
    InvalidPublicKeyLength { expected: usize, got: usize },
    InvalidSignatureLength { expected: usize, got: usize },
    InvalidPublicKeyBytes(String),
    InvalidSignatureBytes(String),
    #[allow(dead_code)]
    VerificationFailed,
}

impl std::fmt::Display for Ed25519VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ed25519VerifyError::InvalidPublicKeyLength { expected, got } => write!(
                f,
                "Invalid Ed25519 public key length: expected {} bytes, got {}",
                expected, got
            ),
            Ed25519VerifyError::InvalidSignatureLength { expected, got } => write!(
                f,
                "Invalid Ed25519 signature length: expected {} bytes, got {}",
                expected, got
            ),
            Ed25519VerifyError::InvalidPublicKeyBytes(msg) => {
                write!(f, "Invalid Ed25519 public key bytes: {}", msg)
            }
            Ed25519VerifyError::InvalidSignatureBytes(msg) => {
                write!(f, "Invalid Ed25519 signature bytes: {}", msg)
            }
            Ed25519VerifyError::VerificationFailed => {
                write!(f, "Ed25519 signature verification failed")
            }
        }
    }
}

impl std::error::Error for Ed25519VerifyError {}

/// Verify an Ed25519 detached signature over `canonical_payload`.
///
/// Parameters:
///   - `canonical_payload`: the exact byte sequence that was signed
///   - `signature_bytes`:   the 64-byte detached Ed25519 signature
///   - `public_key_bytes`:  the 32-byte compressed Ed25519 public key
///
/// Returns:
///   - `Ok(true)`  — signature is valid
///   - `Ok(false)` — signature bytes are structurally valid but do not match
///   - `Err(_)`    — key/signature bytes are malformed
pub fn verify_ed25519(
    canonical_payload: &[u8],
    signature_bytes: &[u8],
    public_key_bytes: &[u8],
) -> Result<bool, Ed25519VerifyError> {
    // Validate public key length.
    if public_key_bytes.len() != 32 {
        return Err(Ed25519VerifyError::InvalidPublicKeyLength {
            expected: 32,
            got: public_key_bytes.len(),
        });
    }

    // Validate signature length.
    if signature_bytes.len() != 64 {
        return Err(Ed25519VerifyError::InvalidSignatureLength {
            expected: 64,
            got: signature_bytes.len(),
        });
    }

    // Construct VerifyingKey from the 32-byte array.
    let pk_array: &[u8; 32] = <&[u8; 32]>::try_from(public_key_bytes)
        .expect("length already checked to be exactly 32");
    let verifying_key = VerifyingKey::from_bytes(pk_array)
        .map_err(|e| Ed25519VerifyError::InvalidPublicKeyBytes(e.to_string()))?;

    // Construct Signature from the raw bytes.
    let signature = Signature::from_slice(signature_bytes)
        .map_err(|e| Ed25519VerifyError::InvalidSignatureBytes(e.to_string()))?;

    // Verify — verify_strict rejects cofactor-malleability.
    match verifying_key.verify_strict(canonical_payload, &signature) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}
