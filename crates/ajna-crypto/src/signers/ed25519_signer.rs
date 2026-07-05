use crate::signer::{AjnaSigner, SignerError};

/// Ed25519 signer backed by a freshly generated ephemeral keypair.
///
/// Each call to [`EdDsaSigner::new()`] generates a new keypair using the OS
/// random number generator. This signer is stateless after construction and
/// is safe to share across threads via `Arc`.
pub struct EdDsaSigner {
    signing_key: ed25519_dalek::SigningKey,
}

impl Default for EdDsaSigner {
    fn default() -> Self {
        Self::new()
    }
}

impl EdDsaSigner {
    /// Generate a new Ed25519 keypair using the OS RNG.
    pub fn new() -> Self {
        use rand::rngs::OsRng;
        Self {
            signing_key: ed25519_dalek::SigningKey::generate(&mut OsRng),
        }
    }
}

impl AjnaSigner for EdDsaSigner {
    fn algorithm_id(&self) -> &'static str {
        "ed25519"
    }

    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError> {
        use ed25519_dalek::Signer;
        let sig = self.signing_key.sign(payload);
        Ok(sig.to_bytes().to_vec())
    }

    fn verify(&self, payload: &[u8], sig: &[u8]) -> Result<bool, SignerError> {
        use ed25519_dalek::{Signature, Verifier};
        let vk = self.signing_key.verifying_key();
        let sig = Signature::from_slice(sig)
            .map_err(|e| SignerError::VerificationFailed(e.to_string()))?;
        vk.verify(payload, &sig)
            .map(|_| true)
            .map_err(|e| SignerError::VerificationFailed(e.to_string()))
    }

    fn public_key_bytes(&self) -> Vec<u8> {
        self.signing_key.verifying_key().to_bytes().to_vec()
    }
}
