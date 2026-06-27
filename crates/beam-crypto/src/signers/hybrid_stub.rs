use crate::signer::{BeamSigner, SignerError};

/// Stub hybrid Ed25519 + ML-DSA-65 signer. Full implementation ships in Phase 2.
pub struct HybridSigner;

impl BeamSigner for HybridSigner {
    fn algorithm_id(&self) -> &'static str {
        "hybrid-ed25519-ml-dsa-65"
    }

    fn sign(&self, _payload: &[u8]) -> Result<Vec<u8>, SignerError> {
        Err(SignerError::NotImplemented(
            "HybridSigner ships in Phase 2".into(),
        ))
    }

    fn verify(&self, _payload: &[u8], _sig: &[u8]) -> Result<bool, SignerError> {
        Err(SignerError::NotImplemented(
            "HybridSigner ships in Phase 2".into(),
        ))
    }

    fn public_key_bytes(&self) -> Vec<u8> {
        vec![]
    }
}
