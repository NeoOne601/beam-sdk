// backend/src/crypto/ml_dsa_verifier.rs
// ML-DSA (CRYSTALS-Dilithium) signature verification for the backend.
//
// Uses pqcrypto-dilithium 0.5 (PQClean Round-3 reference implementation).
// This crate implements CRYSTALS-Dilithium as specified in NIST FIPS 204.
// NOTE: pqcrypto-dilithium has NOT undergone NIST CMVP validation.
// Product-level FIPS validation is on the Phase 3 roadmap.

use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey as _, DetachedSignature as _};

/// Verify a Dilithium-3 (ML-DSA Level 3) detached signature.
///
/// Parameters:
///   - public_key: raw public key bytes (1952 bytes for Dilithium-3)
///   - message: the canonical bytes that were signed
///   - signature: the detached signature (3309 bytes for pqcrypto-dilithium 0.5)
///
/// Returns true if the signature is valid.
///
/// Signature size note:
///   pqcrypto-dilithium 0.5 (PQClean Round-3): 3,309 bytes
///   FIPS 204 ML-DSA-65 final standard: 3,293 bytes
///   These differ because pqcrypto-dilithium uses the pre-final PQClean reference.
pub fn verify_dilithium3(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> bool {
    let pk = match dilithium3::PublicKey::from_bytes(public_key) {
        Ok(pk) => pk,
        Err(_) => {
            tracing::warn!("Invalid Dilithium-3 public key ({} bytes)", public_key.len());
            return false;
        }
    };

    let sig = match dilithium3::DetachedSignature::from_bytes(signature) {
        Ok(sig) => sig,
        Err(_) => {
            tracing::warn!("Invalid Dilithium-3 signature ({} bytes)", signature.len());
            return false;
        }
    };

    match dilithium3::verify_detached_signature(&sig, message, &pk) {
        Ok(_) => true,
        Err(_) => {
            tracing::warn!("Dilithium-3 signature verification failed");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pqcrypto_dilithium::dilithium3;
    use pqcrypto_traits::sign::{PublicKey as _, SecretKey as _, DetachedSignature as _};

    #[test]
    fn test_verify_valid_signature() {
        let (pk, sk) = dilithium3::keypair();
        let message = b"test canonical bytes for verification";
        let sig = dilithium3::detached_sign(message, &sk);

        assert!(verify_dilithium3(
            pk.as_bytes(),
            message,
            sig.as_bytes()
        ));
    }

    #[test]
    fn test_verify_invalid_signature() {
        let (pk, sk) = dilithium3::keypair();
        let message = b"original message";
        let sig = dilithium3::detached_sign(message, &sk);

        // Verify against wrong message
        assert!(!verify_dilithium3(
            pk.as_bytes(),
            b"tampered message",
            sig.as_bytes()
        ));
    }

    #[test]
    fn test_verify_wrong_key() {
        let (_pk1, sk1) = dilithium3::keypair();
        let (pk2, _sk2) = dilithium3::keypair();
        let message = b"signed with key 1";
        let sig = dilithium3::detached_sign(message, &sk1);

        // Verify with wrong public key
        assert!(!verify_dilithium3(
            pk2.as_bytes(),
            message,
            sig.as_bytes()
        ));
    }
}
