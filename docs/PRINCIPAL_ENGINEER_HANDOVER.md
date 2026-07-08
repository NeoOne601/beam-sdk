# Ajna — Principal Engineer Handover

*A brain-dump from the engineer who built this, for the one who takes it over.
Read it once end to end before you touch anything load-bearing. It is not a
rulebook — it is how to think about this system so you don't recreate the
mistakes I already paid for. Where I state an invariant, treat it as a
tripwire: the system is correct **because** of it, and it will break quietly
if you violate it.*

---

## 1. What this system actually is (and what it is not)

Ajna is a **GTM (go-to-market) security platform** with one non-negotiable
property: **result integrity at the edge, provable under a quantum threat
model.** Everything else — the three pillars, the dashboard, the MCP server —
hangs off that spine.

Three product pillars, one shared crypto foundation:

- **Ajna IDV** (`crates/ajna-idv`) — document scanning + identity verification.
- **Ajna Intel** (`crates/ajna-intel`) — device posture / integrity (root,
  jailbreak, hooking frameworks, emulator, debugger) → risk-scored report.
- **Ajna Vision** (`crates/ajna-vision`) — facial liveness (challenge-response
  FSM) + model-agnostic face embedding match.
- **`crates/ajna-crypto`** — the signer registry (Ed25519 + ML-DSA-65) every
  pillar signs through.
- **`crates/ajna-mcp-server`** — exposes the pillars as tools to AI agents.
- **`core/`** (`ajna-core`) — the scanning engine: quality gates, session state
  machine, frame handling, the C FFI boundary, the declarative `UiConfig`.
- **`backend/`** (`ajna-verify-backend`) — Axum service: verification,
  country-rules engine, SOC2 audit chain, NQM server attestation.
- **`dashboard/`** — React/Vite integration portal (onboarding, UI customizer,
  audit viewer).

**What it is NOT, and never claim it is:** a shipped product with millions of
real documents behind it. It is an architecturally complete, test-green
platform with a live free-tier deployment and mobile SDKs that compile. The
on-device ML (OCR, face landmarks) is wired to real native engines (Apple
Vision, ML Kit, MediaPipe) through clean seams; the *parsing and trust logic*
is real and unit-tested against real document formats; the last mile — a
physical phone scanning a real passport — is an integration step, not a
finished, load-tested fact. **If you ever find yourself writing "production-
proven at scale," stop. It isn't, and honesty here is the whole culture.**

---

## 2. The design spine — read this before you change anything

The architecture is not an aesthetic. Every boundary exists to make one thing
independently changeable. Internalize these five and you understand 90% of the
decisions.

### 2.1 The language split is forced by physics, not taste
- **Rust** owns *all* business logic: quality gates, the session FSM, result
  assembly, PQC signing, the FFI declarations. The borrow checker eliminates a
  class of memory bugs before ship, and there is no GC pause at 25 fps.
- **C++** exists at exactly one place: the ML runtime boundary (TFLite / CoreML
  / ONNX). Zero-copy tensor delivery from ISP DRAM to the inference runtime is
  only reachable through C++ APIs. We accept C++ *only* there.
- **Swift / Kotlin** are thin camera adapters: format negotiation, buffer
  pools, HAL variance. **No business logic.** They hand a frame pointer to the
  core and pull JSON back. If you find logic creeping into a platform adapter,
  that's a bug — push it into Rust.

The mobile SDK you actually ship is the **Rust core** (`libajna_core.a` /
`.so`), packaged as an XCFramework (iOS) or AAR (Android). The C++ ML bridge is
a *separate, optional* slice — do not couple the SDK build to it (I learned
this repackaging both; the CMake path that pulls in CoreML is fragile, the
direct `cargo build --target … -p ajna-core` path is not).

### 2.2 Decoupling by environment variable — the reason "deploy anywhere" is cheap
The same backend binary runs against local Docker Postgres, Neon, or Supabase,
changing **nothing but env vars** (`backend/src/db/pool.rs`). Pool size, TLS
mode, timeouts are all env-driven with free-tier-safe defaults. This is why
moving from local → Render+Supabase was a config change, not a rewrite. **Keep
it that way.** The thing that varies per environment must be *data*, never
branching logic. Same principle for the country rules: embedded JSON rulepacks
with an `AJNA_COUNTRY_RULES_PATH` override — policy is configuration, not code.

### 2.3 Illegal states are unrepresentable
The scan pipeline is an explicit enum state machine
(`Idle → Scanning → Inferring → Complete/Failed`). The critical invariant:

> **`AcceptedForInference` is returned in exactly ONE place (`core/src/pipeline.rs`)
> and only when `gate_reached == Gate::Accepted`.** The expensive C++/GPU layer
> is never invoked on a frame the quality gates would have rejected.

This is the performance contract for budget devices *and* a correctness
boundary. The FFI integration test suite asserts it. If you refactor the
pipeline and this stops being single-source, you have introduced a bug even if
every test still passes — add the test that proves the invariant, don't remove
the constraint.

### 2.4 Determinism is load-bearing for signatures
`ScanResult::canonical_bytes()` (`core/src/result.rs`) produces a
deterministic, length-prefixed, lexicographically-sorted byte encoding
*regardless of field insertion order*. The backend's
`reconstruct_canonical_bytes()` mirrors it byte-for-byte. **These two must stay
identical.** If you change the encoding on one side and not the other, every
signature silently fails verification and you will lose a day finding it. When
you touch canonical bytes, change both, and add a cross-check test. The VR-1
nonce/session/timestamp binding lives inside this encoding — that is what makes
a captured signature un-replayable.

### 2.5 The audit log proves its own integrity
`audit_logs` is append-only (DB trigger blocks UPDATE/DELETE) and
**hash-chained per tenant**: `entry_hash = SHA-256(prev_hash | tenant |
session | event_type | outcome | detail)`. `GET /v1/audit/verify-chain`
recomputes the chain and reports tamper. This is the SOC2 Type 2 evidence
mechanism. **Build the "is this still correct?" check into the system** — don't
rely on trusting it.

---

## 3. The bugs that already bit us — do not re-earn these

These all passed local tests and were still real. Each is now a regression
test. They are the texture of this system's failure modes.

1. **`sqlx` had no TLS feature.** Everything worked locally (`sslmode=disable`)
   and failed instantly on Supabase (`TLS upgrade required but SQLx built
   without TLS support`). Fix: `tls-rustls`. **Lesson: local success on the
   untested path is not success.**
2. **`redis` had no TLS feature.** Same class — Upstash `rediss://` needs
   `tokio-rustls-comp`. When you add any managed dependency, check whether its
   client crate was compiled with TLS.
3. **Audit-chain linkage across interleaved rows.** The chain verified fine
   with consecutive verification rows, but `nonce_created` rows (written
   unhashed by the nonce route) interleave by sequence. The "previous hash"
   query grabbed the latest row *by seq* — sometimes an unhashed nonce row —
   and reset to genesis, breaking the chain. It only surfaced after **multiple
   real transactions**. Fix: link to the latest *hashed* row
   (`entry_hash <> ''`). **Lesson: one transaction passes; N interleaved
   transactions expose the bug. Test the plural.**

The meta-lesson: **your tests encode the situations you already imagined.
Verify against reality to catch the ones you didn't** — deploy and hit the real
managed DB, drive several transactions, watch the real metric move (the Upstash
command counter, the `verify-chain` result), not just the health ping.

---

## 4. Security posture — how to think, not just what to check

The mindset: **assume the input is hostile and the boundary is where you
defend.** The remediation log (VR-1..VR-6 in the README) is the canonical
record; here is the reasoning to carry forward.

- **Never trust a client-supplied trust anchor.** `/v1/verify` looks up the
  verification key from a registered table keyed by tenant (the `KeyProvider`
  strategy) — it does not verify against a key the client sent, because that
  proves only internal consistency, not provenance. Any auth where the attacker
  supplies both sides is theatre.
- **Bind the signature to the session.** Nonce + session_id + timestamp are
  inside the signed canonical bytes (VR-1). The backend re-derives and checks
  all three before consuming the nonce. This is what stops replay.
- **Secrets never touch git, logs, or chat.** They live in the platform's env
  UI. In this repo the committed configs hold placeholders only; the JWT secret
  and DB password went into Render's UI by hand. If you are ever about to
  commit a token — even to a private repo — stop. That reflex is not optional.
- **SSRF on webhooks is real** (VR-5): URLs are parsed and rejected if they
  resolve to RFC-1918, loopback, link-local, or cloud metadata addresses; the
  HTTP client has a timeout and a redirect cap.
- **Key memory hygiene** (VR-6): private keys are ephemeral (fresh per session),
  `mlock`'d where the platform allows, and volatile-zeroed on `Drop`. `mlock`
  failure is logged, not fatal — it is defence-in-depth on top of ephemerality
  + zeroing, not the primary control.
- **PQC is a threat-horizon decision, not fashion.** Identity data signed today
  must remain verifiable past 2035; ECDSA/Ed25519 fall to Shor's. ML-DSA-65
  (FIPS 204) is the hedge. The registry keeps Ed25519 for compatibility and
  negotiates per session — crypto-agility is the NQM requirement and the right
  posture regardless.
- Every FFI export null-guards its handles, validates frame dimensions before
  any unsafe read, and returns `i32` status codes — a panic across the FFI
  boundary is UB (VR-4). Keep it that way; every `unsafe` block carries a
  `SAFETY:` comment stating the invariant.

---

## 5. Operating it — the runbook you'll actually need

**Local dev (no cloud):**
```bash
# Postgres + Redis local, backend on :8080
brew services start postgresql@16 redis   # or the launch you already have
export DATABASE_URL="postgres://ajna:ajna@localhost:5432/ajna_verify?sslmode=disable"
export REDIS_URL="redis://127.0.0.1:6379" DB_REQUIRE_TLS=false
cargo run --release -p ajna-verify-backend -j 2
curl localhost:8080/health   # {"status":"ok","db":"ok","redis":"ok"}
```

**Cloud (the live free-tier stack):** backend on Render
(`deploy/render.yaml` Blueprint — API + free Key Value Redis, `REDIS_URL`
auto-wired), Postgres on Supabase, dashboard on Vercel. Secrets set in each
platform's UI. The runbooks are in `deploy/DEPLOYMENT.md`, `deploy/DEPLOY_NOW.md`,
`deploy/LIVE_STACK.md`. **Free-tier caveats you must communicate:** Render
sleeps after ~15 min idle (first request cold-starts ~50s); a Cloudflare
quick-tunnel URL is ephemeral (dies with the process). These are demo/staging
economics, not a production SLA — say so.

**Mobile builds** (`deploy/MOBILE_BUILD.md`): iOS →
`scripts/package_ios_xcframework.sh` → `dist/AjnaSDK.xcframework`. Android →
`scripts/package_android_aar.sh` (needs `ANDROID_NDK`) → the AAR, then Gradle →
APK. Device install + Xcode signing is the user's step — state it and pause.

**The hardware constraint is a real design input.** The build host is an 8GB
M1. That is why every `cargo` call uses `-j 2`, builds run sequentially, LTO is
`thin` not `fat`, and containers cap at 256MB. "It works on a big machine" is
not "it works." Respect the envelope; it will OOM if you don't.

---

## 6. Verification discipline — how "done" is defined here

Before you claim anything works, name the observation that proves it, and make
sure it happened. The house standard:

- Run the *exact* command the spec names, with its flags:
  `RUSTFLAGS="-D warnings" cargo test --release -j 2` — zero warnings is part of
  the contract, not a nicety. Current suite: ~146 tests green, clippy clean.
- Prove negatives concretely: "no leaks" = `leaks --atExit → 0 bytes` on the
  FFI test binary; "chain intact" = the endpoint returns `valid:true` over
  *several interleaved* writes.
- Exercise the real path: Redis "ok" in `/health` is a ping; whether *your*
  Redis serves is the command counter moving. The cloud DB path is not the
  local DB path — test the one that ships.
- When a test and reality disagree, reality wins; fix the code and add the test
  that would have caught it. That is how the suite grows past what you already
  knew.

---

## 7. Extending it safely — the patterns to copy

- **New pillar / capability** → new crate that depends on `ajna-core` +
  `ajna-crypto`, exposing a product-shaped facade (see `ajna-idv`). Sign
  results through the shared registry so provenance is uniform. Add it to the
  MCP server if agents should call it.
- **New document type** → extend `ajna-idv`'s OCR parser (`ocr.rs`) with the
  format's field extraction + its integrity check (Verhoeff for Aadhaar, ICAO
  check digits for passports, AAMVA for US DL). The pixel→text step stays a
  pluggable native engine; you own the *parse + validate* logic, and it must be
  unit-tested against real layouts.
- **New country policy** → edit `backend/src/rules/country_rules.json`. Do not
  write `if country == …` in a handler.
- **New signing algorithm** → register it in `ajna-crypto`'s `SignerRegistry`;
  the negotiation endpoint and canonical-bytes path already handle multiple
  algorithms.
- Match the surrounding code's style exactly. Many small focused files over few
  large ones. Comments state the *why* / the constraint / the upgrade path,
  never restate the code. Immutable by default. Errors explicit, never
  swallowed. Every non-trivial branch leaves one runnable check behind.

---

## 8. Roadmap & known debt — where the bodies are

- **On-device ML is wired, not battle-tested.** Apple Vision (iOS) and ML Kit +
  MediaPipe (Android) are integrated at real seams; a physical-device pass with
  real IDs and faces across a device matrix is the next real milestone. Treat
  current mobile status as "compiles + typechecks + one-device smoke," not
  "field-proven."
- **The OCR→Rust FFI seam** exists conceptually (the Swift/Kotlin apps hint
  fields client-side and POST to the backend); wiring the OCR text lines
  *through* the `ajna-idv` `DocumentParser` over a dedicated FFI entry, so
  Verhoeff/ICAO validation runs in Rust on-device, is the clean next step. The
  parser is ready; the FFI export isn't there yet.
- **Free-tier deployment ≠ production.** For production: dedicated Postgres,
  `min_machines_running > 0` to kill cold starts, real secret management, an
  observability stack (the `tracing` spans are there — wire them to a backend).
- **The C++ ML bridge** (CoreML/TFLite) is the fragile build path. If you need
  real on-device inference in the SDK slice, invest in making that build
  reproducible rather than bolting it onto the Rust-core packaging.
- `dump.rdb` is a stray Redis dump that keeps showing dirty in git status —
  gitignore it and move on.

---

## 9. The one thing to carry above all

The job is not to produce output that looks finished. It is to make the true
state of the system match what's needed and to report that state honestly —
especially when the honest answer is "this part compiles and is tested, this
part is deployed on free tier, and this part needs a physical device I don't
have." A truthful gap beats a confident overclaim every time. This system's
credibility — quantum-safe integrity, tamper-evident audit — is *entirely* a
trust property. Do not undermine it with a single fabricated "verified." Build
the check, run it, report what it said.

Good luck. Read the invariants in §2 twice, keep §3 pinned above your desk, and
verify more than feels necessary.
