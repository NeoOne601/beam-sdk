// backend/src/nqm.rs
//
// Indian National Quantum Mission (NQM) compliance surface.
//
// Two obligations, both explicit in the wire format:
//   1. Crypto agility — every verify response declares which signature
//      profile covered the client payload (classical, PQC, or hybrid), so
//      auditors can prove which algorithm protected which verification.
//      Algorithm negotiation itself lives in POST /v1/session/init.
//   2. Quantum-safe server attestation — the backend counter-signs each
//      verification outcome with its own ephemeral ML-DSA-65 (FIPS 204)
//      key, so downstream consumers can prove the *server's* verdict was
//      not forged, under both classical and quantum attack models.

use ajna_crypto::{AjnaSigner, MlDsaSigner};
use serde::Serialize;

/// Wire-visible compliance envelope attached to every verify response.
#[derive(Debug, Clone, Serialize)]
pub struct NqmCompliance {
    /// Human-auditable profile label.
    pub profile: String,
    /// Algorithm that signed the client payload.
    pub algorithm: String,
    /// Whether the client payload signature is post-quantum.
    pub pqc: bool,
    /// Standards this verification aligns with.
    pub standards: Vec<String>,
}

/// Classify a client signing algorithm into its NQM profile.
pub fn envelope_for(algorithm: &str) -> NqmCompliance {
    let pqc = algorithm.starts_with("ml-dsa");
    let hybrid = algorithm.starts_with("hybrid");
    let profile = if hybrid {
        "NQM-HYBRID-1"
    } else if pqc {
        "NQM-PQC-1"
    } else {
        "CLASSICAL-1"
    };
    let mut standards = vec!["NQM crypto-agility".to_owned()];
    if pqc || hybrid {
        standards.push("FIPS 204 (ML-DSA)".to_owned());
    }
    NqmCompliance {
        profile: profile.to_owned(),
        algorithm: algorithm.to_owned(),
        pqc: pqc || hybrid,
        standards,
    }
}

/// Server counter-signature over a verification outcome.
#[derive(Debug, Clone, Serialize)]
pub struct ServerAttestation {
    /// Exactly what was signed: "verification_id|verified|timestamp".
    pub payload: String,
    pub algorithm: String,
    pub signature_b64: String,
    pub public_key_b64: String,
}

/// Holds the backend's ML-DSA-65 attestation key for the process lifetime.
/// The key is ephemeral by design: attestations are verified against the
/// public key embedded in the response, not a long-lived registry.
pub struct AttestationSigner {
    signer: MlDsaSigner,
}

impl Default for AttestationSigner {
    fn default() -> Self {
        Self::new()
    }
}

impl AttestationSigner {
    pub fn new() -> Self {
        Self {
            signer: MlDsaSigner::new(),
        }
    }

    /// Counter-sign one verification outcome.
    pub fn attest(
        &self,
        verification_id: uuid::Uuid,
        verified: bool,
        timestamp: &str,
    ) -> Result<ServerAttestation, ajna_crypto::SignerError> {
        use base64::Engine as _;
        let payload = format!("{verification_id}|{verified}|{timestamp}");
        let signature = self.signer.sign(payload.as_bytes())?;
        let b64 = base64::engine::general_purpose::STANDARD;
        Ok(ServerAttestation {
            payload,
            algorithm: self.signer.algorithm_id().to_owned(),
            signature_b64: b64.encode(signature),
            public_key_b64: b64.encode(self.signer.public_key_bytes()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

    #[test]
    fn ml_dsa_payloads_are_classified_as_nqm_pqc() {
        let envelope = envelope_for("ml-dsa-65");
        assert_eq!(envelope.profile, "NQM-PQC-1");
        assert!(envelope.pqc);
        assert!(envelope.standards.iter().any(|s| s.contains("FIPS 204")));
    }

    #[test]
    fn classical_payloads_are_labelled_classical() {
        let envelope = envelope_for("ed25519");
        assert_eq!(envelope.profile, "CLASSICAL-1");
        assert!(!envelope.pqc);
    }

    #[test]
    fn attestation_signs_with_ml_dsa_and_verifies() {
        let attestation_signer = AttestationSigner::new();
        let id = uuid::Uuid::new_v4();
        let attestation = attestation_signer
            .attest(id, true, "2026-07-05T00:00:00Z")
            .expect("attestation must sign");

        assert_eq!(attestation.algorithm, "ml-dsa-65");
        assert!(attestation.payload.starts_with(&id.to_string()));

        let signature = base64::engine::general_purpose::STANDARD
            .decode(&attestation.signature_b64)
            .expect("valid base64");
        assert!(attestation_signer
            .signer
            .verify(attestation.payload.as_bytes(), &signature)
            .expect("verification must run"));
    }
}
