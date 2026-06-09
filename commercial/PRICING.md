# Beam Verify — Pricing

## Pricing Tiers

All prices in USD. Billed monthly. Volume discounts available on annual contracts.

| Tier | Monthly Price | Verifications/mo | Support | SLA |
|------|--------------|-------------------|---------|-----|
| **Startup** | $499 | Up to 5,000 | Email (48h) | 99.5% |
| **Growth** | $1,999 | Up to 50,000 | Email (24h) + Slack | 99.9% |
| **Enterprise** | Custom | Unlimited | Dedicated SE + 4h response | 99.99% |

## What's Included (All Tiers)

- Beam Verify SDK (Android, iOS, Web/WASM)
- Post-quantum cryptographic signing (ML-DSA Level 3)
- On-device quality gate pipeline
- Backend verification service
- Admin dashboard
- CycloneDX SBOM and threat model documentation
- Standard document types: passport, driving licence, national ID

## Enterprise Add-Ons

| Add-On | Price |
|--------|-------|
| Custom document type training | $15,000 one-time |
| On-premise backend deployment | $5,000/mo |
| Dedicated model fine-tuning | $25,000 one-time |
| FIPS 140-3 CMVP validation support | $50,000 (Phase 3 roadmap) |
| Compliance documentation package | Included with Enterprise |

## SDK-Only License

For integrators who operate their own verification backend:

| Plan | Price | Includes |
|------|-------|----------|
| SDK-Only | $299/mo | SDK binaries, model updates, schema access |
| SDK + Support | $799/mo | SDK-Only + technical support + integration assistance |

## Overage Pricing

| Tier | Per-verification overage |
|------|------------------------|
| Startup | $0.15 |
| Growth | $0.08 |
| Enterprise | Custom (negotiated) |

## Free Trial

- 30-day free trial with up to 500 verifications
- Full SDK access (all platforms)
- Sample apps included
- No credit card required

## Billing Notes

- All prices exclude applicable taxes
- Annual contracts receive 15% discount
- Verifications counted at the backend `/v1/verify` endpoint
- Failed verifications (signature invalid) are not counted
