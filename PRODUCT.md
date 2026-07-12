# PRODUCT.md — Ajna Product Vision & GTM

> Scorecard file (CLAUDE.md §3). Living document. Last updated: 2026-07-12 (cycle 1).

## North Star (10–15 year horizon)

Ajna is **AI-native digital trust infrastructure**, not an identity-verification vendor.
Today's IDV market (document scan + selfie match) is an intermediate stage. The durable
market is **verifiable trust between humans, devices, AI agents, and content** under two
irreversible shifts:

1. **Synthetic media becomes free** — deepfakes make "what the camera saw" worthless
   without cryptographic capture-time attestation and liveness that resists injection.
2. **AI agents become economic actors** — agents will transact, sign, and delegate.
   They need verifiable identity, scoped delegation chains, and audit trails exactly
   like the ones Ajna already builds for humans (PQC signing, hash-chained audit, MCP).

Everything we ship for IDV today (edge PQC signing, tamper-evident audit, agent-facing
MCP tools, crypto agility) is deliberately reusable as the substrate for that market.

## Positioning

**One line:** Verifiable identity, device, and liveness signals whose integrity holds
under a post-quantum threat model — deployable for $0, scalable to enterprise by
changing environment variables, and consumable by both apps and AI agents.

**Category:** Digital trust infrastructure (IDV + device posture + liveness + audit).

**Wedge:** Regulated Indian fintech (NQM/DPDP alignment is a moat no Western vendor
has), expanding to global regulated industries via SOC2-out-of-the-box + PQC readiness.

## ICP (initial)

- **Who:** CTO / Head of Risk at a Series A–C fintech, neobank, lending, or insurance
  company in India or serving Indian users; 10k–5M verifications/year.
- **Pains:** KYC vendor lock-in, opaque audit trails at SOC2/RBI audit time, deepfake
  onboarding fraud, looming PQC compliance (NQM), slow vendor integrations (weeks).
- **Buying trigger:** audit finding, fraud spike, or new regulatory requirement.

Secondary ICP: AI-application platforms needing verification tools inside agent stacks
(MCP-native — no other IDV vendor ships this today).

## Value Proposition

| Pillar | What the buyer gets |
|---|---|
| Ajna IDV | On-device document OCR + validation (Aadhaar Verhoeff, ICAO MRZ, AAMVA DL) — PII can stay on device |
| Ajna Vision | Challenge-response liveness FSM with anti-replay; signed results |
| Ajna Intel | Device posture / root / hook detection → risk-scored signed reports |
| Compliance | SOC2-style tamper-evident hash-chained audit log out of the box; NQM-aligned PQC attestation |
| Integration | 60-minute onboarding portal; iOS/Android/WASM SDKs; MCP server for AI agents |

## GTM

1. **Developer-led:** $0 self-serve tier (Render/Supabase/Vercel reference stack),
   60-minute integration flow, open documentation.
2. **Compliance-led:** SOC2/NQM/DPDP evidence packs generated from the audit chain —
   sell to the auditor's checklist, not just the developer.
3. **Agent-led (differentiator):** listed in MCP registries; agents can call
   `ajna_verify_document` / `ajna_verify_face` / `ajna_evaluate_device_posture` /
   `ajna_query_audit_log` as tools.

## Pricing Model (draft — Human Gate before publishing)

- **Free:** 500 verifications/mo, community support, Ajna watermark.
- **Growth:** per-verification (volume-tiered, ~₹4–₹12 / $0.05–$0.15 by pillar mix),
  removes watermark, email support.
- **Enterprise:** annual platform fee + committed volume; device-level key strategy,
  private deployment (env-var swap to own Postgres/Redis), SLAs, DPA.

Pricing publication is a §13 Human Gate (business approval required).

## Competitive Landscape (differentiation matrix)

| Capability | **Ajna** | Jumio | Onfido (Entrust) | iProov | Sumsub | Persona | HyperVerge (IN) |
|---|---|---|---|---|---|---|---|
| On-device processing (PII stays local) | ✅ Rust core | ❌ cloud | ❌ cloud | ❌ cloud | ❌ cloud | ❌ cloud | Partial |
| PQC edge signing (FIPS 204 ML-DSA) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Tamper-evident hash-chained audit (SOC2 evidence) | ✅ built-in | Partial | Partial | ❌ | Partial | Partial | ❌ |
| AI-agent integration (MCP tools) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| India Stack docs (Aadhaar Verhoeff etc.) | ✅ | Partial | Partial | ❌ | ✅ | ❌ | ✅ |
| NQM / India PQC alignment | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Liveness anti-injection maturity | 🔶 FSM + motion; needs device-matrix hardening | ✅ | ✅ | ✅ (best) | ✅ | ✅ | ✅ |
| Self-serve $0 start | ✅ | ❌ | ❌ | ❌ | Partial | ✅ | ❌ |

Honest read: incumbents beat us today on presentation-attack-detection maturity and
document coverage breadth. We win on architecture (edge + PQC + agent-native + audit),
which is where the 10-year market moves. Close the PAD gap; don't chase doc coverage
breadth first.

## Success Criterion Mapping (§12)

- Criterion 1 (product vision): this file. Status: **YES (v1, evolve each cycle)**.
- Criterion 11 (≥5 competitors + matrix): table above. Status: **YES (deepen with cited research in a research cycle)**.
