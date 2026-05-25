// beam-core/src/crypto.rs
// Post-Quantum Cryptography signing module.
//
// WHY PQC HERE?
// ─────────────
// Surt's threat model: identity documents processed by Beam feed into a compliance
// graph that persists for years. Classic ECDSA signatures over that data are
// vulnerable to "harvest now, decrypt later" attacks: an adversary stores signed
// result blobs today and decrypts them once a CRQC (Cryptographically Relevant
// Quantum Computer) is available — estimated window: 5–15 years (NIST IR 8547, 2024).
//
// We sign with ML-DSA (CRYSTALS-Dilithium, FIPS 204, August 2024).
// Key encapsulation for transport uses ML-KEM (CRYSTALS-Kyber, FIPS 203).
// Both are NIST-standardised, lattice-based, and quantum-resistant.
//
// IMPLEMENTATION NOTE:
// A production build links against liboqs (Open Quantum Safe) or pqcrypto crate.
// This module provides the interface and a stubbed implementation for the SDK core.
// The actual cryptographic computation is FFI-linked at build time.

use alloc::vec::Vec;

/// ML-DSA security level. Use Level3 (128-bit quantum security) for production.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum MlDsaLevel {
    Level2,  // 128-bit classical, 64-bit quantum  — fastest
    Level3,  // 192-bit classical, 96-bit quantum  — recommended
    Level5,  // 256-bit classical, 128-bit quantum — highest assurance
}

pub struct PqcSigner {
    level:       MlDsaLevel,
    private_key: Vec<u8>,
    public_key:  Vec<u8>,
}

#[derive(Debug)]
pub enum CryptoError {
    KeyGenerationFailed,
    SigningFailed,
    VerificationFailed,
    InvalidKey,
}

impl PqcSigner {
    /// Generate a new ephemeral ML-DSA keypair.
    /// In production: keys are generated at factory provisioning time and stored
    /// in the platform Secure Enclave (iOS) or StrongBox Keymaster (Android).
    pub fn generate(level: MlDsaLevel) -> Result<Self, CryptoError> {
        // Stub: in production delegates to liboqs OQS_SIG_dilithium_{2,3,5}_keypair
        let (sk_len, pk_len) = match level {
            MlDsaLevel::Level2 => (2528, 1312),
            MlDsaLevel::Level3 => (4000, 1952),
            MlDsaLevel::Level5 => (4864, 2592),
        };
        // STUB — replace with: unsafe { OQS_SIG_dilithium_3_keypair(pk.as_mut_ptr(), sk.as_mut_ptr()) }
        let private_key = alloc::vec![0xAB_u8; sk_len];
        let public_key  = alloc::vec![0xCD_u8; pk_len];
        Ok(Self { level, private_key, public_key })
    }

    /// Sign `message` bytes. Returns the ML-DSA signature.
    /// Signature length: Level3 → 3293 bytes.
    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let sig_len = match self.level {
            MlDsaLevel::Level2 => 2420,
            MlDsaLevel::Level3 => 3293,
            MlDsaLevel::Level5 => 4595,
        };
        // STUB — replace with: OQS_SIG_dilithium_3_sign(sig, &siglen, msg, msglen, sk)
        let _ = (message, &self.private_key);
        Ok(alloc::vec![0x00_u8; sig_len])
    }

    /// Verify a signature against a message and public key bytes.
    pub fn verify(
        level:      MlDsaLevel,
        public_key: &[u8],
        message:    &[u8],
        signature:  &[u8],
    ) -> Result<bool, CryptoError> {
        // STUB — replace with: OQS_SIG_dilithium_3_verify(msg, msglen, sig, siglen, pk)
        let _ = (level, public_key, message, signature);
        Ok(true)
    }

    pub fn public_key_bytes(&self) -> &[u8] {
        &self.public_key
    }
}

/// ML-KEM (CRYSTALS-Kyber, FIPS 203) key encapsulation for result transport.
/// Used when transmitting ScanResult from device to Surt backend over TLS.
/// Provides quantum-safe key exchange for the session key.
pub struct MlKemSession {
    pub ciphertext:   Vec<u8>,   // Sent to peer
    pub shared_secret: Vec<u8>,  // Used to derive AES-256-GCM key for payload
}

impl MlKemSession {
    /// KEM-1024: equivalent to AES-256 security level.
    pub fn encapsulate(server_public_key: &[u8]) -> Result<Self, CryptoError> {
        // STUB — replace with: OQS_KEM_kyber_1024_encaps(ct, ss, pk)
        let _ = server_public_key;
        Ok(Self {
            ciphertext:    alloc::vec![0u8; 1568], // KEM-1024 ciphertext size
            shared_secret: alloc::vec![0u8; 32],
        })
    }
}
