# tasks/todo.md — Current Cycle Scorecard

## Cycle 1 (2026-07-12) — Bootstrap + Dashboard Enterprise Overhaul (§15)

- [x] Bootstrap state files: PRODUCT, ARCHITECTURE, RESEARCH, DECISIONS, ROADMAP, lessons
- [x] Install §15 deps: react-router-dom, recharts, lucide-react (pre-approved by §15)
- [x] Dashboard overhaul D1–D14 (auth, routing, telemetry, charts, palette, toasts,
      skeletons, responsive, a11y, onboarding polish, audit drill-down, customizer,
      key management)
- [x] `npm run build` green (tsc + vite)
- [x] Browser verification pass (desktop/tablet/mobile, console clean)
- [x] §9 verifier agent: PASS — all D1–D14 YES, visual audit PREMIUM; MEDIUM
      scrim-resize defect + metric rounding fixed and re-verified
- [x] Update memory.md, commit (conventional commits)

### Carried from verifier (LOW, next dashboard cycle)
- [ ] Focus trap + focus-restore in ConfirmDialog / CommandPalette
- [ ] `Hash` chip keyboard operability (tabIndex + Enter)
- [ ] useCountUp freezes in background tabs (rAF) — acceptable; revisit if reported
- [ ] Code-split recharts chunk (640 kB bundle) if load time matters

## Backlog (carried)

### Mobile compile & wire (blocked on §13 Human Gates — owner installs)
- Android NDK r26 + SDK + Gradle → AAR/APK; emscripten → WASM; ngrok.
- iOS: Vision wiring + xcframework typecheck; Xcode signing for device deploy.
- MediaPipe `face_landmarker.task` → samples/android assets.
- Supabase seed verify (`ajna_live_sk_demo_0000`); local backend :8080.

### Phase 1 queue (see ROADMAP.md)
- OpenAPI spec (§12.7) · anti-injection strategy doc (§12.4) · PQC hybrid design
  (§12.5) · identity model consolidation (§12.6) · docs freshness sweep (§12.9).
