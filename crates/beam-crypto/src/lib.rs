pub mod jws;
pub mod signer;
pub mod signers;

pub use signer::{BeamSigner, SignerError, SignerRegistry};
pub use signers::*;

use std::sync::{OnceLock, RwLock};

/// Process-wide signer registry, lazily initialised on first access.
static GLOBAL_REGISTRY: OnceLock<RwLock<SignerRegistry>> = OnceLock::new();

/// Return a reference to the global [`SignerRegistry`].
///
/// The registry is lazily created (empty) on first call. To pre-populate it,
/// call [`init_registry`] before any signing occurs.
pub fn global_registry() -> &'static RwLock<SignerRegistry> {
    GLOBAL_REGISTRY.get_or_init(|| RwLock::new(SignerRegistry::new()))
}

/// Set the global [`SignerRegistry`] to a fully-configured instance.
///
/// # Panics
///
/// Panics if the global registry has already been initialised (either by a
/// previous `init_registry` call or by a lazy `global_registry()` access).
/// This is intentional — double-init is a programming error that should be
/// caught early.
pub fn init_registry(registry: SignerRegistry) {
    if GLOBAL_REGISTRY.set(RwLock::new(registry)).is_err() {
        panic!("init_registry called twice");
    }
}
