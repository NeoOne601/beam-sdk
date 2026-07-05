use crate::signer::{AjnaSigner, SignerError};
use std::boxed::Box;

/// A flexible hybrid signer that combines two cryptographic signers.
///
/// It generates length-prefixed binary concatenations of the signatures
/// and public keys produced by both inner signers.
pub struct HybridSigner {
    id: &'static str,
    classical: Box<dyn AjnaSigner + Send + Sync>,
    pqc: Box<dyn AjnaSigner + Send + Sync>,
}

impl HybridSigner {
    /// Create a new HybridSigner wrapping two underlying signers.
    pub fn new(
        id: &'static str,
        classical: Box<dyn AjnaSigner + Send + Sync>,
        pqc: Box<dyn AjnaSigner + Send + Sync>,
    ) -> Self {
        Self { id, classical, pqc }
    }
}

impl AjnaSigner for HybridSigner {
    fn algorithm_id(&self) -> &'static str {
        self.id
    }

    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError> {
        let sig1 = self.classical.sign(payload)?;
        let sig2 = self.pqc.sign(payload)?;

        let mut out = Vec::with_capacity(8 + sig1.len() + sig2.len());
        out.extend_from_slice(&(sig1.len() as u32).to_le_bytes());
        out.extend_from_slice(&sig1);
        out.extend_from_slice(&(sig2.len() as u32).to_le_bytes());
        out.extend_from_slice(&sig2);
        Ok(out)
    }

    fn verify(&self, payload: &[u8], sig: &[u8]) -> Result<bool, SignerError> {
        if sig.len() < 8 {
            return Err(SignerError::VerificationFailed(
                "Hybrid signature too short".into(),
            ));
        }

        // Parse classical signature
        let mut offset = 0;
        let len1 = u32::from_le_bytes(sig[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if sig.len() < offset + len1 + 4 {
            return Err(SignerError::VerificationFailed(
                "Hybrid signature length mismatch".into(),
            ));
        }
        let sig1 = &sig[offset..offset + len1];
        offset += len1;

        // Parse PQC signature
        let len2 = u32::from_le_bytes(sig[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if sig.len() < offset + len2 {
            return Err(SignerError::VerificationFailed(
                "Hybrid signature length mismatch".into(),
            ));
        }
        let sig2 = &sig[offset..offset + len2];

        // Verify both
        let v1 = self.classical.verify(payload, sig1)?;
        let v2 = self.pqc.verify(payload, sig2)?;
        Ok(v1 && v2)
    }

    fn public_key_bytes(&self) -> Vec<u8> {
        let pk1 = self.classical.public_key_bytes();
        let pk2 = self.pqc.public_key_bytes();

        let mut out = Vec::with_capacity(8 + pk1.len() + pk2.len());
        out.extend_from_slice(&(pk1.len() as u32).to_le_bytes());
        out.extend_from_slice(&pk1);
        out.extend_from_slice(&(pk2.len() as u32).to_le_bytes());
        out.extend_from_slice(&pk2);
        out
    }
}
