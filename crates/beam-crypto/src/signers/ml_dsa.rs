#[cfg(feature = "pqc")]
use crate::signer::{BeamSigner, SignerError};

/// ML-DSA-65 (Dilithium-3) signer backed by pqcrypto-dilithium.
///
/// Only compiled when the `pqc` feature is enabled.
#[cfg(feature = "pqc")]
pub struct MlDsaSigner {
    public_key: pqcrypto_dilithium::dilithium3::PublicKey,
    secret_key: pqcrypto_dilithium::dilithium3::SecretKey,
}

#[cfg(feature = "pqc")]
impl MlDsaSigner {
    /// Generate a fresh ML-DSA-65 keypair.
    pub fn new() -> Self {
        let (pk, sk) = pqcrypto_dilithium::dilithium3::keypair();
        Self {
            public_key: pk,
            secret_key: sk,
        }
    }
}

#[cfg(feature = "pqc")]
impl Default for MlDsaSigner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "pqc")]
impl BeamSigner for MlDsaSigner {
    fn algorithm_id(&self) -> &'static str {
        "ml-dsa-65"
    }

    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError> {
        use pqcrypto_dilithium::dilithium3::detached_sign;
        use pqcrypto_traits::sign::DetachedSignature;
        let sig = detached_sign(payload, &self.secret_key);
        Ok(sig.as_bytes().to_vec())
    }

    fn verify(&self, payload: &[u8], sig: &[u8]) -> Result<bool, SignerError> {
        use pqcrypto_dilithium::dilithium3::{verify_detached_signature, DetachedSignature};
        use pqcrypto_traits::sign::DetachedSignature as DetachedSignatureTrait;
        let sig_obj = DetachedSignature::from_bytes(sig)
            .map_err(|e| SignerError::VerificationFailed(format!("{:?}", e)))?;
        verify_detached_signature(&sig_obj, payload, &self.public_key)
            .map(|_| true)
            .map_err(|e| SignerError::VerificationFailed(format!("{:?}", e)))
    }

    fn public_key_bytes(&self) -> Vec<u8> {
        use pqcrypto_traits::sign::PublicKey;
        self.public_key.as_bytes().to_vec()
    }
}
