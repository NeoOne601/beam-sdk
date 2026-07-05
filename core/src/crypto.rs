// core/src/crypto.rs
// Post-Quantum Cryptography signing module.
//
// WHY PQC HERE?
// ─────────────
// Ajna's threat model: identity documents processed by Ajna feed into a compliance
// graph that persists for years. Classic ECDSA signatures over that data are
// vulnerable to "harvest now, decrypt later" attacks: an adversary stores signed
// result blobs today and decrypts them once a CRQC (Cryptographically Relevant
// Quantum Computer) is available — estimated window: 5–15 years (NIST IR 8547, 2024).
//
// We sign with ML-DSA (CRYSTALS-Dilithium, FIPS 204, August 2024).
// Key encapsulation for transport uses ML-KEM (CRYSTALS-Kyber, FIPS 203).
// Both are NIST-standardised, lattice-based, and quantum-resistant.
//
// IMPLEMENTATION:
// This module uses the pqcrypto-dilithium and pqcrypto-kyber crates.
// Private key bytes are mlock()-protected on non-WASM targets to prevent
// them from being swapped to disk.

use pqcrypto_traits::sign::{DetachedSignature as _, PublicKey as _, SecretKey as _};
use std::vec::Vec;

/// ML-DSA security level. Use Level3 (128-bit quantum security) for production.
///
/// IMPORTANT NOTE on signature sizes and FIPS 204 compliance:
/// ───────────────────────────────────────────────────────────
/// This implementation uses `pqcrypto-dilithium` v0.5, which wraps PQClean's
/// Round-3 submission of Dilithium. PQClean reports CRYPTO_BYTES = 3309 for
/// Dilithium-3, whereas the finalised FIPS 204 standard (August 2024) specifies
/// 3293 bytes for ML-DSA-87 (Level3).
///
/// The 16-byte discrepancy arises from differences in encoding:
/// - PQClean Round-3: includes additional hint bytes in signature encoding
/// - FIPS 204 final: uses a more compact encoding without those hints
///
/// COMPLIANCE STATUS:
/// • Functionally: Cryptographically equivalent to FIPS 204 ML-DSA-87
/// • Wire format: NOT byte-for-byte compatible with FIPS 204 canonical form
/// • Risk: May fail interoperability tests with strict FIPS 204 validators
///
/// TODO: When a FIPS 204-certified Rust crate becomes available (e.g., from
///       PQClean's FIPS branch or NIST reference implementation), migrate to
///       ensure exact byte-level compliance. Track: https://github.com/PQClean/PQClean/issues/XXX
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum MlDsaLevel {
    Level2, // 128-bit classical, 64-bit quantum  — fastest
    Level3, // 192-bit classical, 96-bit quantum  — recommended (Dilithium-3)
    Level5, // 256-bit classical, 128-bit quantum — highest assurance
}

pub struct PqcSigner {
    level: MlDsaLevel,
    private_key: Vec<u8>,
    public_key: Vec<u8>,
}

#[derive(Debug)]
pub enum CryptoError {
    KeyGenerationFailed,
    SigningFailed,
    VerificationFailed,
    InvalidKey,
}

impl PqcSigner {
    /// Generate a new ML-DSA keypair.
    ///
    /// In production: keys are generated at factory provisioning time and stored
    /// in the platform Secure Enclave (iOS) or StrongBox Keymaster (Android).
    /// On WASM: stored in memory only — document this limitation to integrators.
    pub fn generate(level: MlDsaLevel) -> Result<Self, CryptoError> {
        match level {
            MlDsaLevel::Level2 => {
                use pqcrypto_dilithium::dilithium2;
                let (pk, sk) = dilithium2::keypair();
                let mut private_key = sk.as_bytes().to_vec();
                let public_key = pk.as_bytes().to_vec();
                Self::mlock_key(&mut private_key);
                Ok(Self {
                    level,
                    private_key,
                    public_key,
                })
            }
            MlDsaLevel::Level3 => {
                use pqcrypto_dilithium::dilithium3;
                let (pk, sk) = dilithium3::keypair();
                let mut private_key = sk.as_bytes().to_vec();
                let public_key = pk.as_bytes().to_vec();
                Self::mlock_key(&mut private_key);
                Ok(Self {
                    level,
                    private_key,
                    public_key,
                })
            }
            MlDsaLevel::Level5 => {
                use pqcrypto_dilithium::dilithium5;
                let (pk, sk) = dilithium5::keypair();
                let mut private_key = sk.as_bytes().to_vec();
                let public_key = pk.as_bytes().to_vec();
                Self::mlock_key(&mut private_key);
                Ok(Self {
                    level,
                    private_key,
                    public_key,
                })
            }
        }
    }

    /// Lock private key memory pages to prevent swap (non-WASM only).
    ///
    /// VR-6 (Security): mlock return value is now checked.
    /// If mlock fails the key is still used — signing remains correct — but a
    /// warning is emitted so operators can fix the RLIMIT_MEMLOCK limit.
    /// See README.md §VR-6 for full rationale on Option B (warn, continue).
    #[cfg(all(feature = "mlock", not(target_arch = "wasm32")))]
    fn mlock_key(key: &mut [u8]) {
        // Safety: key.as_ptr() is valid for key.len() bytes.
        let rc = unsafe { libc::mlock(key.as_ptr() as *const _, key.len()) };
        if rc != 0 {
            // mlock failed — key may be swappable to disk.
            // Common causes: RLIMIT_MEMLOCK quota, container without CAP_IPC_LOCK,
            // or platform sandbox restriction. Signing continues; this is defense-in-depth.
            // Operator action: raise RLIMIT_MEMLOCK or add --cap-add IPC_LOCK.
            eprintln!(
                "[ajna-core] WARNING: mlock failed for private key ({} bytes). \
                 Key may be swappable to disk. Raise RLIMIT_MEMLOCK to suppress. \
                 Error code: {}",
                key.len(),
                rc
            );
        }
    }

    #[cfg(not(all(feature = "mlock", not(target_arch = "wasm32"))))]
    fn mlock_key(_key: &mut [u8]) {
        // no-op on WASM — document: private key is in-memory only on WASM targets
    }

    /// Sign `message` bytes. Returns the raw ML-DSA detached signature.
    ///
    /// Signature lengths (PQClean Round-3 / FIPS 204 final):
    ///   - Level2: 2420 bytes / 2420 bytes (identical)
    ///   - Level3: 3309 bytes / 3293 bytes (16-byte discrepancy, see MlDsaLevel docs)
    ///   - Level5: 4595 bytes / 4595 bytes (identical)
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, CryptoError> {
        match self.level {
            MlDsaLevel::Level2 => {
                use pqcrypto_dilithium::dilithium2::{detached_sign, SecretKey};
                let sk = SecretKey::from_bytes(&self.private_key)
                    .map_err(|_| CryptoError::SigningFailed)?;
                let sig = detached_sign(message, &sk);
                Ok(sig.as_bytes().to_vec())
            }
            MlDsaLevel::Level3 => {
                use pqcrypto_dilithium::dilithium3::{detached_sign, SecretKey};
                let sk = SecretKey::from_bytes(&self.private_key)
                    .map_err(|_| CryptoError::SigningFailed)?;
                let sig = detached_sign(message, &sk);
                Ok(sig.as_bytes().to_vec())
            }
            MlDsaLevel::Level5 => {
                use pqcrypto_dilithium::dilithium5::{detached_sign, SecretKey};
                let sk = SecretKey::from_bytes(&self.private_key)
                    .map_err(|_| CryptoError::SigningFailed)?;
                let sig = detached_sign(message, &sk);
                Ok(sig.as_bytes().to_vec())
            }
        }
    }

    /// Verify a detached signature against a message and public key bytes.
    pub fn verify(
        level: MlDsaLevel,
        public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<bool, CryptoError> {
        match level {
            MlDsaLevel::Level2 => {
                use pqcrypto_dilithium::dilithium2::{
                    verify_detached_signature, DetachedSignature, PublicKey,
                };
                let pk = PublicKey::from_bytes(public_key).map_err(|_| CryptoError::InvalidKey)?;
                let sig = DetachedSignature::from_bytes(signature)
                    .map_err(|_| CryptoError::InvalidKey)?;
                Ok(verify_detached_signature(&sig, message, &pk).is_ok())
            }
            MlDsaLevel::Level3 => {
                use pqcrypto_dilithium::dilithium3::{
                    verify_detached_signature, DetachedSignature, PublicKey,
                };
                let pk = PublicKey::from_bytes(public_key).map_err(|_| CryptoError::InvalidKey)?;
                let sig = DetachedSignature::from_bytes(signature)
                    .map_err(|_| CryptoError::InvalidKey)?;
                Ok(verify_detached_signature(&sig, message, &pk).is_ok())
            }
            MlDsaLevel::Level5 => {
                use pqcrypto_dilithium::dilithium5::{
                    verify_detached_signature, DetachedSignature, PublicKey,
                };
                let pk = PublicKey::from_bytes(public_key).map_err(|_| CryptoError::InvalidKey)?;
                let sig = DetachedSignature::from_bytes(signature)
                    .map_err(|_| CryptoError::InvalidKey)?;
                Ok(verify_detached_signature(&sig, message, &pk).is_ok())
            }
        }
    }

    pub fn public_key_bytes(&self) -> &[u8] {
        &self.public_key
    }
}

impl Drop for PqcSigner {
    fn drop(&mut self) {
        // Zero the private key before deallocation.
        // Safety: self.private_key is valid for its length.
        for byte in &mut self.private_key {
            // Volatile write prevents compiler from optimising this away.
            unsafe { core::ptr::write_volatile(byte, 0) };
        }
        #[cfg(all(feature = "mlock", not(target_arch = "wasm32")))]
        unsafe {
            // VR-6: Check munlock return value and warn on failure.
            let rc = libc::munlock(
                self.private_key.as_ptr() as *const _,
                self.private_key.len(),
            );
            if rc != 0 {
                eprintln!(
                    "[ajna-core] WARNING: munlock failed for private key ({} bytes). \
                     Error code: {}",
                    self.private_key.len(),
                    rc
                );
            }
        }
    }
}

/// ML-KEM (CRYSTALS-Kyber, FIPS 203) key encapsulation for result transport.
/// Used when transmitting ScanResult from device to Ajna backend over TLS.
/// Provides quantum-safe key exchange for the session key.
pub struct MlKemSession {
    /// Ciphertext sent to the server. Length: Kyber-1024 → 1568 bytes.
    pub ciphertext: Vec<u8>,
    /// Shared secret used to derive AES-256-GCM key. Length: 32 bytes.
    pub shared_secret: Vec<u8>,
}

impl MlKemSession {
    /// KEM-1024 encapsulation: equivalent to AES-256 security level.
    ///
    /// Ciphertext length: 1568 bytes.
    /// Shared secret length: 32 bytes.
    pub fn encapsulate(server_public_key: &[u8]) -> Result<Self, CryptoError> {
        use pqcrypto_kyber::kyber1024::{encapsulate, PublicKey};
        use pqcrypto_traits::kem::{Ciphertext as _, PublicKey as _, SharedSecret as _};
        let pk = PublicKey::from_bytes(server_public_key).map_err(|_| CryptoError::InvalidKey)?;
        let (ss, ct) = encapsulate(&pk);
        Ok(Self {
            ciphertext: ct.as_bytes().to_vec(),
            shared_secret: ss.as_bytes().to_vec(),
        })
    }
}
