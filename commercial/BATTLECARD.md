# Beam Verify — Competitive Battlecard

## Positioning Statement

Beam Verify is the first identity verification SDK with **post-quantum cryptographic signing** built into the on-device pipeline. While competitors sign results with classical ECDSA (broken by quantum computers), Beam uses ML-DSA (NIST FIPS 204) to future-proof every scan result against the harvest-now, decrypt-later threat.

---

## Competitive Matrix

| Capability | Beam Verify | Scandit ID Bolt | Microblink BlinkID | Veriff |
|-----------|-------------|-----------------|-------------------|--------|
| **PQC Signing (ML-DSA)** | ✅ Yes | ❌ No | ❌ No | ❌ No |
| **On-device inference** | ✅ TFLite/CoreML/ONNX | ✅ Proprietary | ✅ Proprietary | ❌ Server-side |
| **Quality gates (pre-inference)** | ✅ 4-stage CPU pipeline | ⚠️ Basic blur check | ⚠️ Basic checks | ❌ N/A (server) |
| **Zero-copy frame path** | ✅ AHardwareBuffer/CVPixelBuffer | ❌ Copy-based | ⚠️ Partial | ❌ N/A |
| **Open output schema** | ✅ Published JSON schema | ❌ Proprietary | ❌ Proprietary | ❌ Proprietary |
| **WASM/Web support** | ✅ ONNX Runtime | ⚠️ Limited | ✅ Yes | ✅ Web SDK |
| **Backend verification** | ✅ Included (Axum) | ❌ Separate | ❌ Separate | ✅ Included |
| **SBOM provided** | ✅ CycloneDX 1.4 | ❌ No | ❌ No | ❌ No |
| **Threat model published** | ✅ Yes | ❌ No | ❌ No | ⚠️ Partial |
| **Performance budget** | < 4ms gates, < 45ms inference | Not published | Not published | N/A |
| **Starting price** | $499/mo | ~$1,500/mo | ~$999/mo | ~$299/mo + per-check |

## Key Differentiators

### 1. Post-Quantum Security (Unique)
No competitor offers NIST-standardised post-quantum signatures. This is critical for:
- Financial services (long-term compliance value of KYC data)
- Government contracts (NIST PQC migration mandates)
- Healthcare (HIPAA long-term data protection)

### 2. Transparent Security Model
Published SBOM, threat model, export control documentation, and performance budgets. Competitors treat these as trade secrets.

### 3. Performance-First Architecture
4-stage quality gate pipeline rejects bad frames in < 4ms on a Cortex-A55 — before any GPU work. This saves battery and reduces thermal throttling on budget devices.

### 4. Open Schema
The `output_schema.json` is the single source of truth for model ↔ SDK contract. Integrators know exactly what fields are available and their formats. No proprietary black-box output.

## Common Objections

| Objection | Response |
|-----------|----------|
| "We already use Scandit/BlinkID" | Those SDKs sign with ECDSA. When quantum computers arrive (2030–2040), those signatures can be forged. Every KYC result signed today is at risk. |
| "PQC isn't required yet" | NIST published FIPS 204 in August 2024. Federal agencies have migration mandates. Early adopters avoid the rush. |
| "Beam is newer, less proven" | The Rust core has zero `unsafe` outside of FFI boundaries. The C++ layer is limited to inference only. Published benchmarks and threat model demonstrate maturity. |
| "Price is higher than Veriff" | Veriff is server-side processing — you're sending raw face images to their servers. Beam processes on-device with PQC signing. Different trust model. |
| "We need liveness detection" | Beam Verify pairs with FaceGuard (Surt AI's face biometric product). Beam handles document scanning; FaceGuard handles liveness and face match. |

## Target Segments

1. **Fintech / Neobanks** — KYC compliance with future-proof cryptography
2. **Government / Defense** — NIST PQC migration mandates
3. **Healthcare** — HIPAA-compliant identity verification
4. **Crypto / Web3** — Regulatory KYC with on-device processing (privacy)
5. **Travel & Hospitality** — Passport scanning with MRZ validation
