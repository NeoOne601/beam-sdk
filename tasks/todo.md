# Ajna Live Demo — Deploy, Mobile, Edge ML Spec

Goal: take the verified Ajna workspace to a deployable, demonstrable state — edge
ML seams, mobile sample source, free-tier hosting configs, Palantir dashboard.

## Environment reality (what this autonomous shell can vs cannot do)

Verified locally (build + test here):
- Rust edge-ML logic, connection pooling, dashboard build, config/YAML validity.

CANNOT be done in this non-interactive shell — needs the user's machine/accounts:
- **Actual cloud deploy** (cond 8/10): no docker/flyctl/vercel CLIs, no accounts,
  no interactive OAuth. → Deliver configs + scripts + exact runbook instead.
- **Run mobile app on a device** (cond 11): no Android SDK, no physical device,
  no camera, no human. → Deliver buildable source + on-device run instructions.
- **Real Aadhaar/Passport/DL scan + live face** (cond 1/11): no camera, no
  physical documents, no human face. → Deliver the parsing + landmark→gesture
  logic (the verifiable half) behind a pluggable native OCR/landmark engine.

The honest split: I build and verify all the **software seams**; the pixel→text
OCR model, the MediaPipe landmark extractor, the cloud deploy, and the physical
capture are wired to exact integration points and documented for you to run.

## Assumptions
- `-j 2` on every cargo call; no release LTO rebuild loops; no heavy installs
  (Tesseract/Docker/Android SDK) — they can't be verified here anyway.
- Edge ML = pluggable trait + real parsing/derivation logic. Native engines
  (Tesseract/Paddle, MediaPipe FaceMesh) implement the trait on-device.

## Tasks

### Part 1 — Edge ML seams (cond 1,2,3)
- [ ] ajna-idv `ocr` module: `OcrEngine` trait (pixels→text lines) + `DocumentParser`
      that structures lines into fields for Aadhaar / Passport-MRZ / US-DL (AAMVA),
      → `ScanResult`, signed via ajna-crypto ML-DSA-65. Real, tested parsing.
- [ ] ajna-vision `landmarks` module: FaceLandmarks → gesture observations
      (blink via eye-aspect-ratio, turn via yaw, smile via mouth-aspect-ratio)
      feeding the existing liveness FSM. Real, tested derivation.
- [ ] Sign OCR result + liveness verdict with ML-DSA-65 (reuse pillar signing).
- [ ] Gate: `cargo test -p ajna-idv -p ajna-vision -j 2`

### Part 3 — Hosting & DB (cond 6,7,8,9)  [before Part 2: infra unblocks nothing downstream]
- [ ] backend `db::pool`: env-configurable sqlx PgPool (max/min conns, acquire/
      idle timeouts, sslmode) for Neon/Supabase serverless; wire into main.rs.
- [ ] Dashboard Dockerfile (multi-stage build → static serve).
- [ ] Root `docker-compose.yml`: api + dashboard + db + redis, `mem_limit: 256m`.
- [ ] Deploy configs: `deploy/fly.toml`, `deploy/render.yaml`, `dashboard/vercel.json`,
      `deploy/DEPLOYMENT.md` runbook (Neon + Fly/Render + Vercel, $0 tier).
- [ ] Dashboard Palantir restyle: deep-slate HUD theme, monospaced hashes,
      F-pattern layout, progressive-disclosure `<details>` drawers for hashes/JSON.
- [ ] Gate: `cd dashboard && npm run build`

### Part 2 — Mobile sample source (cond 4,5)
- [ ] `examples/android-sample`: Kotlin/Compose scaffold — CameraX, UiConfig overlay,
      liveness FSM call sites, JNI seam to ajna-core, backend POST. Source only.
- [ ] `examples/ios-sample`: SwiftUI scaffold — AVFoundation, UiConfig overlay,
      ajna-core FFI seam, backend POST. Source only.
- [ ] READMEs with on-device build/run steps (the parts needing SDK + device).

### Part 4 — Verify (cond 12 + honest status)
- [ ] `cargo test --release -j 2` full workspace, zero warnings; clippy clean.
- [ ] Dashboard builds; screenshot the Palantir UI via preview.
- [ ] Final report: done-and-verified vs blocked-on-your-resources.

## Review

### Built & verified here
- **Edge ML (cond 1,2,3):** ajna-idv `ocr` (Aadhaar/Verhoeff, passport MRZ/ICAO
  check digits, US-DL/AAMVA) + `scan_and_sign` → ML-DSA-65. ajna-vision
  `landmarks` (EAR blink, yaw turn, MAR smile) → liveness FSM. 37 tests pass.
- **DB pooling (cond 7):** backend `db::pool` env-tuned for Neon/Supabase,
  wired into main.rs. 3 pool tests pass.
- **Docker/compose (cond 6):** dashboard Dockerfile, backend Dockerfile fixed
  for repo-root context, root docker-compose.yml @ 256m limits.
- **Deploy configs (cond 8):** fly.toml, render.yaml, vercel.json, DEPLOYMENT.md.
- **Palantir dashboard (cond 9):** tactical HUD theme, hash chips, progressive
  disclosure drawers, F-pattern. `npm run build` clean; verified in preview.
- **Mobile source (cond 4,5):** Android Compose + iOS SwiftUI scaffolds.
- **Tests (cond 12):** `cargo test --release -j 2` — 145 pass, 0 warnings,
  clippy clean.

### Blocked on user resources (cannot be done in an autonomous shell)
- **cond 8/10 actual deploy + live server:** needs your Fly/Render/Vercel/Neon
  accounts + interactive OAuth. Run `deploy/DEPLOYMENT.md`.
- **cond 11 device run + live face/doc:** needs a physical device, camera, a
  human face, and real Aadhaar/Passport/DL. Build via the example READMEs.
- **cond 1 "real documents" end-to-end:** the parsing is verified against real
  layouts; the pixel→text OCR model runs on-device (native engine seam).
