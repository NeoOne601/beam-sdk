use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during cryptographic signing or verification.
#[derive(Debug, Error)]
pub enum SignerError {
    #[error("signing failed: {0}")]
    SigningFailed(String),

    #[error("verification failed: {0}")]
    VerificationFailed(String),

    #[error("not implemented: {0}")]
    NotImplemented(String),

    #[error("invalid key: {0}")]
    InvalidKey(String),
}

/// Trait representing a pluggable signing algorithm.
///
/// Implementations provide key generation, signing, and verification for a
/// specific algorithm (e.g. Ed25519, ML-DSA-65). The trait is object-safe so
/// that heterogeneous signers can be stored behind `Arc<dyn BeamSigner>`.
pub trait BeamSigner: Send + Sync {
    /// A unique, stable identifier for this algorithm (e.g. `"ed25519"`,
    /// `"ml-dsa-65"`). Used as the key in [`SignerRegistry`].
    fn algorithm_id(&self) -> &'static str;

    /// Sign `payload`, returning the raw signature bytes.
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, SignerError>;

    /// Verify that `sig` is a valid signature of `payload` under this signer's
    /// public key.
    fn verify(&self, payload: &[u8], sig: &[u8]) -> Result<bool, SignerError>;

    /// Return the raw public-key bytes for this signer instance.
    fn public_key_bytes(&self) -> Vec<u8>;
}

/// A registry of available [`BeamSigner`] implementations.
///
/// Signers are stored keyed by their `algorithm_id()` and a preferred ordering
/// is maintained so that callers can request "the best available" signer via
/// [`preferred()`](SignerRegistry::preferred).
pub struct SignerRegistry {
    signers: HashMap<String, Arc<dyn BeamSigner>>,
    preferred_order: Vec<String>,
}

impl Default for SignerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SignerRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            signers: HashMap::new(),
            preferred_order: Vec::new(),
        }
    }

    /// Register a signer. The signer's `algorithm_id()` is used as the map key
    /// and is appended to the preferred-order list.
    ///
    /// Returns `&mut Self` so calls can be chained:
    /// ```ignore
    /// registry.register(Ed25519Signer::generate()?)
    ///         .register(MlDsaSigner::generate()?);
    /// ```
    pub fn register(&mut self, signer: impl BeamSigner + 'static) -> &mut Self {
        let id = signer.algorithm_id().to_owned();
        self.signers.insert(id.clone(), Arc::new(signer));
        self.preferred_order.push(id);
        self
    }

    /// Look up a signer by algorithm identifier. Returns `None` (never panics)
    /// if the algorithm is not registered.
    pub fn select(&self, algo_id: &str) -> Option<Arc<dyn BeamSigner>> {
        self.signers.get(algo_id).cloned()
    }

    /// Return the first signer in the preferred order that is still present in
    /// the registry, or `None` if the registry is empty.
    pub fn preferred(&self) -> Option<Arc<dyn BeamSigner>> {
        self.preferred_order
            .iter()
            .find_map(|id| self.signers.get(id).cloned())
    }

    /// Return the list of registered algorithm identifiers in preferred order.
    pub fn supported_algorithms(&self) -> Vec<String> {
        self.preferred_order.clone()
    }
}
