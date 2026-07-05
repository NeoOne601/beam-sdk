// Face template matching over model-agnostic embedding vectors.
//
// Ajna Vision does not bundle a face model: the platform's inference bridge
// (CoreML / TFLite / ONNX — same zero-copy path as IDV) produces an f32
// embedding, and this module owns the comparison math and its edge cases.

use std::fmt;

/// Default cosine-similarity acceptance threshold. Tuned per deployment via
/// the backend country/tenant rules; this is the SDK-side fallback.
pub const DEFAULT_MATCH_THRESHOLD: f32 = 0.75;

#[derive(Debug, Clone, PartialEq)]
pub enum MatchError {
    EmptyEmbedding,
    NonFiniteValue,
    DimensionMismatch { left: usize, right: usize },
    ZeroNorm,
}

impl fmt::Display for MatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEmbedding => write!(f, "embedding is empty"),
            Self::NonFiniteValue => write!(f, "embedding contains NaN or infinity"),
            Self::DimensionMismatch { left, right } => {
                write!(f, "embedding dimensions differ: {left} vs {right}")
            }
            Self::ZeroNorm => write!(f, "embedding has zero magnitude"),
        }
    }
}

impl std::error::Error for MatchError {}

/// An L2-comparable face embedding. Construction validates the vector once
/// so comparison code never re-checks.
#[derive(Debug, Clone, PartialEq)]
pub struct FaceEmbedding(Vec<f32>);

impl FaceEmbedding {
    pub fn new(values: Vec<f32>) -> Result<Self, MatchError> {
        if values.is_empty() {
            return Err(MatchError::EmptyEmbedding);
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err(MatchError::NonFiniteValue);
        }
        if values.iter().map(|v| v * v).sum::<f32>() == 0.0 {
            return Err(MatchError::ZeroNorm);
        }
        Ok(Self(values))
    }

    pub fn dimension(&self) -> usize {
        self.0.len()
    }

    /// Cosine similarity in `[-1.0, 1.0]`.
    pub fn cosine_similarity(&self, other: &Self) -> Result<f32, MatchError> {
        if self.0.len() != other.0.len() {
            return Err(MatchError::DimensionMismatch {
                left: self.0.len(),
                right: other.0.len(),
            });
        }
        let dot: f32 = self.0.iter().zip(&other.0).map(|(a, b)| a * b).sum();
        let norm_left: f32 = self.0.iter().map(|v| v * v).sum::<f32>().sqrt();
        let norm_right: f32 = other.0.iter().map(|v| v * v).sum::<f32>().sqrt();
        // Norms are non-zero by construction.
        Ok((dot / (norm_left * norm_right)).clamp(-1.0, 1.0))
    }

    /// True when similarity to `other` meets `threshold`.
    pub fn matches(&self, other: &Self, threshold: f32) -> Result<bool, MatchError> {
        Ok(self.cosine_similarity(other)? >= threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_embeddings_have_similarity_one() {
        let a = FaceEmbedding::new(vec![0.3, -0.5, 0.8]).unwrap();
        let sim = a.cosine_similarity(&a).unwrap();
        assert!((sim - 1.0).abs() < 1e-6, "got {sim}");
    }

    #[test]
    fn orthogonal_embeddings_have_similarity_zero() {
        let a = FaceEmbedding::new(vec![1.0, 0.0]).unwrap();
        let b = FaceEmbedding::new(vec![0.0, 1.0]).unwrap();
        let sim = a.cosine_similarity(&b).unwrap();
        assert!(sim.abs() < 1e-6, "got {sim}");
    }

    #[test]
    fn opposite_embeddings_have_similarity_minus_one() {
        let a = FaceEmbedding::new(vec![0.5, 0.5]).unwrap();
        let b = FaceEmbedding::new(vec![-0.5, -0.5]).unwrap();
        let sim = a.cosine_similarity(&b).unwrap();
        assert!((sim + 1.0).abs() < 1e-6, "got {sim}");
    }

    #[test]
    fn dimension_mismatch_is_rejected() {
        let a = FaceEmbedding::new(vec![1.0, 0.0]).unwrap();
        let b = FaceEmbedding::new(vec![1.0, 0.0, 0.0]).unwrap();
        assert_eq!(
            a.cosine_similarity(&b),
            Err(MatchError::DimensionMismatch { left: 2, right: 3 })
        );
    }

    #[test]
    fn invalid_embeddings_are_rejected_at_construction() {
        assert_eq!(FaceEmbedding::new(vec![]), Err(MatchError::EmptyEmbedding));
        assert_eq!(
            FaceEmbedding::new(vec![f32::NAN]),
            Err(MatchError::NonFiniteValue)
        );
        assert_eq!(
            FaceEmbedding::new(vec![0.0, 0.0]),
            Err(MatchError::ZeroNorm)
        );
    }

    #[test]
    fn matches_applies_threshold() {
        let a = FaceEmbedding::new(vec![1.0, 0.1]).unwrap();
        let b = FaceEmbedding::new(vec![1.0, 0.0]).unwrap();
        assert!(a.matches(&b, DEFAULT_MATCH_THRESHOLD).unwrap());
        assert!(!a.matches(&b, 0.9999).unwrap());
    }
}
