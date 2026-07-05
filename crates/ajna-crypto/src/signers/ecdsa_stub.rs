use crate::signer::{AjnaSigner, SignerError};

/// Stub ECDSA-P256 signer. Full implementation ships in Phase 2.
pub struct EcdsaSigner;

impl AjnaSigner for EcdsaSigner {
    fn algorithm_id(&self) -> &'static str {
        "ecdsa-p256"
    }

    fn sign(&self, _payload: &[u8]) -> Result<Vec<u8>, SignerError> {
        Err(SignerError::NotImplemented(
            "EcdsaSigner ships in Phase 2".into(),
        ))
    }

    fn verify(&self, _payload: &[u8], _sig: &[u8]) -> Result<bool, SignerError> {
        Err(SignerError::NotImplemented(
            "EcdsaSigner ships in Phase 2".into(),
        ))
    }

    fn public_key_bytes(&self) -> Vec<u8> {
        vec![]
    }
}
