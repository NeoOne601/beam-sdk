# Beam Verify SDK — Export Control Notice

## EAR Classification

The Beam Verify SDK incorporates cryptographic functionality subject to the U.S. Export Administration Regulations (EAR), 15 CFR Parts 730–774.

### Applicable ECCN

**ECCN 5D002.c.1** — Software using or performing cryptographic functions employing algorithms with key lengths exceeding 56 bits for symmetric algorithms or 512 bits for asymmetric algorithms.

### Cryptographic Algorithms Used

| Algorithm | Standard | Key Length | ECCN Basis |
|-----------|----------|-----------|------------|
| ML-DSA (CRYSTALS-Dilithium) Level 3 | NIST FIPS 204 | 4000-byte secret key (128-bit PQ security) | 5A002.a.1 equivalent |
| ML-KEM (CRYSTALS-Kyber) 1024 | NIST FIPS 203 | Shared secret: 256 bits (AES-256-GCM) | 5A002.a.1 equivalent |
| AES-256-GCM | NIST FIPS 197 | 256-bit | 5A002.a.1 |
| Ed25519 | RFC 8032 | 256-bit | 5A002.a.1 |
| HMAC-SHA256 | NIST FIPS 198-1 | 256-bit | Excepted — authentication only |

### License Exception ENC (§740.17)

This product is eligible for License Exception ENC (§740.17(b)(1)) as a mass-market encryption product, provided:

1. A self-classification report is filed with BIS (Bureau of Industry and Security) and the ENC Encryption Request Coordinator at the NSA.
2. The encryption functionality is limited to authentication, digital signature, and key management (no general-purpose encryption of user data beyond transport security).

### Filing Requirements

- **ENC self-classification**: Must be filed within 30 days of first export or re-export.
- **Annual self-classification report**: Due by February 1 of each year.
- **BIS contact**: enc@bis.doc.gov
- **SNAP-R filing**: Required for Supplement No. 8 to Part 742.

### Sanctioned Destinations

The SDK must NOT be exported to, or used by nationals of, countries under comprehensive U.S. sanctions (currently: Cuba, Iran, North Korea, Syria, and the Crimea, Donetsk, and Luhansk regions of Ukraine).

### Integrator Responsibility

Integrators incorporating Beam Verify into their applications are independently responsible for their own EAR compliance, including any additional classification requirements arising from their specific product configuration or end-use.

## Out of Scope

- Wassenaar Arrangement multilateral export controls (national implementation varies)
- EU Dual-Use Regulation (EU 2021/821)
- UK Strategic Export Controls
- Individual country import restrictions on PQC

These are the integrator's responsibility based on their specific export destinations.
