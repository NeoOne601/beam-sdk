// ajna-vision — Ajna Vision: facial liveness and face verification.
//
// The platform's inference bridge produces face embeddings and per-frame
// gesture confidences (same zero-copy ML path as IDV). This crate owns the
// trust logic: the liveness challenge FSM, the embedding comparison math,
// and the signed `VisionResult` that carries the verdict to the backend
// with the same ajna-crypto PQC provenance as every other Ajna pillar.

pub mod landmarks;
pub mod liveness;
pub mod matcher;

pub use landmarks::{FaceLandmarks, GestureThresholds, LandmarkError, Point3};
pub use liveness::{
    Challenge, ChallengeObservation, FailReason, LivenessConfig, LivenessConfigError,
    LivenessSession, LivenessState, SubmitOutcome,
};
pub use matcher::{FaceEmbedding, MatchError, DEFAULT_MATCH_THRESHOLD};

use ajna_crypto::{AjnaSigner, SignerError};
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisionVerdict {
    Verified,
    LivenessFailed,
    FaceMismatch,
}

/// The signed-payload output of a face verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionResult {
    pub verdict: VisionVerdict,
    pub liveness_passed: bool,
    pub challenges_completed: usize,
    /// Cosine similarity between probe and reference, when both were
    /// compared (absent if liveness already failed).
    pub match_score: Option<f32>,
    pub threshold: f32,
    /// Unix seconds, supplied by the caller for determinism.
    pub evaluated_at_unix: u64,
}

/// Combine a finished liveness session with an embedding comparison into a
/// final verdict. Liveness gates matching: a failed liveness session never
/// reaches the face comparison (no oracle for probe harvesting).
pub fn verify_face(
    session: &LivenessSession,
    probe: &FaceEmbedding,
    reference: &FaceEmbedding,
    threshold: f32,
    evaluated_at_unix: u64,
) -> Result<VisionResult, MatchError> {
    let liveness_passed = session.state() == LivenessState::Passed;
    if !liveness_passed {
        return Ok(VisionResult {
            verdict: VisionVerdict::LivenessFailed,
            liveness_passed: false,
            challenges_completed: session.challenges_completed(),
            match_score: None,
            threshold,
            evaluated_at_unix,
        });
    }

    let score = probe.cosine_similarity(reference)?;
    let verdict = if score >= threshold {
        VisionVerdict::Verified
    } else {
        VisionVerdict::FaceMismatch
    };
    Ok(VisionResult {
        verdict,
        liveness_passed: true,
        challenges_completed: session.challenges_completed(),
        match_score: Some(score),
        threshold,
        evaluated_at_unix,
    })
}

/// A vision result plus detached signature and provenance material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedVisionResult {
    /// The exact JSON bytes that were signed.
    pub result_json: String,
    /// ajna-crypto algorithm identifier (e.g. "ed25519", "ml-dsa-65").
    pub algorithm: String,
    pub signature_b64: String,
    pub public_key_b64: String,
}

impl VisionResult {
    /// Deterministic JSON payload (fixed struct field order).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("VisionResult serialization cannot fail")
    }

    /// Sign this result with any signer from the shared ajna-crypto registry.
    pub fn sign(&self, signer: &dyn AjnaSigner) -> Result<SignedVisionResult, SignerError> {
        let result_json = self.to_json();
        let signature = signer.sign(result_json.as_bytes())?;
        let b64 = base64::engine::general_purpose::STANDARD;
        Ok(SignedVisionResult {
            result_json,
            algorithm: signer.algorithm_id().to_owned(),
            signature_b64: b64.encode(signature),
            public_key_b64: b64.encode(signer.public_key_bytes()),
        })
    }
}

impl SignedVisionResult {
    /// Verify the detached signature with the signer that produced it.
    ///
    /// Normalizes the ajna-crypto contract (which reports an unsatisfied
    /// verification equation as `Err(VerificationFailed)`) into a plain
    /// boolean: `Ok(false)` means "signature invalid", errors are reserved
    /// for malformed input.
    pub fn verify_with(&self, signer: &dyn AjnaSigner) -> Result<bool, SignerError> {
        let b64 = base64::engine::general_purpose::STANDARD;
        let Ok(signature) = b64.decode(&self.signature_b64) else {
            return Ok(false);
        };
        match signer.verify(self.result_json.as_bytes(), &signature) {
            Ok(valid) => Ok(valid),
            Err(SignerError::VerificationFailed(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ajna_crypto::EdDsaSigner;

    const NOW: u64 = 1_751_700_000;

    fn passed_session() -> LivenessSession {
        let mut session = LivenessSession::new(LivenessConfig::default()).unwrap();
        session.start(1_000);
        session.submit(ChallengeObservation {
            challenge: Challenge::Blink,
            confidence: 0.95,
            timestamp_us: 2_000,
        });
        session.submit(ChallengeObservation {
            challenge: Challenge::TurnLeft,
            confidence: 0.95,
            timestamp_us: 3_000,
        });
        assert_eq!(session.state(), LivenessState::Passed);
        session
    }

    #[test]
    fn same_face_with_passed_liveness_is_verified() {
        let embedding = FaceEmbedding::new(vec![0.2, 0.4, 0.6]).unwrap();
        let result = verify_face(
            &passed_session(),
            &embedding,
            &embedding,
            DEFAULT_MATCH_THRESHOLD,
            NOW,
        )
        .unwrap();
        assert_eq!(result.verdict, VisionVerdict::Verified);
        assert!(result.match_score.unwrap() > 0.99);
    }

    #[test]
    fn different_face_is_a_mismatch() {
        let probe = FaceEmbedding::new(vec![1.0, 0.0]).unwrap();
        let reference = FaceEmbedding::new(vec![0.0, 1.0]).unwrap();
        let result = verify_face(
            &passed_session(),
            &probe,
            &reference,
            DEFAULT_MATCH_THRESHOLD,
            NOW,
        )
        .unwrap();
        assert_eq!(result.verdict, VisionVerdict::FaceMismatch);
    }

    #[test]
    fn failed_liveness_never_reaches_face_comparison() {
        let session = LivenessSession::new(LivenessConfig::default()).unwrap(); // never started
        let embedding = FaceEmbedding::new(vec![1.0, 0.0]).unwrap();
        let result = verify_face(
            &session,
            &embedding,
            &embedding,
            DEFAULT_MATCH_THRESHOLD,
            NOW,
        )
        .unwrap();
        assert_eq!(result.verdict, VisionVerdict::LivenessFailed);
        assert_eq!(result.match_score, None);
    }

    #[test]
    fn signed_result_verifies_and_tamper_is_rejected() {
        let signer = EdDsaSigner::new();
        let embedding = FaceEmbedding::new(vec![0.2, 0.4, 0.6]).unwrap();
        let result = verify_face(
            &passed_session(),
            &embedding,
            &embedding,
            DEFAULT_MATCH_THRESHOLD,
            NOW,
        )
        .unwrap();
        let mut signed = result.sign(&signer).expect("signing must succeed");
        assert!(signed.verify_with(&signer).expect("verify must run"));

        signed.result_json = signed.result_json.replace("verified", "face_mismatch");
        assert!(!signed.verify_with(&signer).expect("verify must run"));
    }
}
