# Ajna Mobile Compile & Wire — Plan

## Toolchain reality (recon done)
| Need | Status | Blocker |
|---|---|---|
| Rust apple + android targets | ✅ installed | — |
| Xcode 26.6 | ✅ | — |
| Android NDK | ❌ missing | ~1GB install (needs permission) |
| Android SDK + Gradle | ❌ missing | multi-GB (needs permission; 8GB RAM risk) |
| emscripten (WASM) | ❌ missing | ~1GB install (needs permission) |
| ngrok | ❌ missing | install + authtoken signup |
| local Postgres/Redis | ✅ installed | — |
| Supabase (migrated+seeded) | ✅ done prior goal | verify only |

## Do now (NO new installs)
- [ ] 1. Download MediaPipe `face_landmarker.task` → `samples/android/app/src/main/assets/` (criterion 1a).
- [ ] 2. iOS: build Rust core for aarch64-apple-ios + aarch64-apple-ios-sim (`-j 2`),
      package `dist/AjnaSDK.xcframework` via scripts/package_ios_xcframework.sh (criterion 2b). VERIFY it builds.
- [ ] 3. iOS: wire liveness to Vision (VNDetectFaceLandmarksRequest), OCR to Vision
      (VNRecognizeTextRequest), AVFoundation frames; replace simulated ScanView.swift flow
      with real AjnaSDK.swift / AjnaCameraAdapter.swift calls (criterion 1b, 4).
- [ ] 4. Android SOURCE wiring (compiles once NDK/SDK present): ML Kit Text Recognition
      dep, MediaPipe landmarker, real JNI calls in ScanActivity.kt → AjnaNativeBridge (criterion 3).
      (APK build itself gated on SDK install.)
- [ ] 5. Supabase: verify migrations + `ajna_live_sk_demo_0000` seed (criterion 5a).
- [ ] 6. Local backend on :8080 (criterion 5b, partial — WASM gated on emscripten).
- [ ] 7. iOS compile verification: swiftc typecheck of sample against the xcframework headers
      (full Xcode build needs an .xcodeproj + signing — state steps, pause) (criterion 6b).

## Needs your permission (install) — will ask, then resume
- Android NDK (r26) — for android Rust targets + AAR (criterion 2a).
- Android SDK cmdline-tools + platform + build-tools + Gradle — for APK (criterion 6a).
- emscripten — for WASM/JS adapter (criterion 5b WASM).
- ngrok — to expose backend+web to your phone (criterion 5c); also needs your authtoken.

## Needs your manual setup (state + pause)
- Xcode signing / provisioning profile for physical iPhone deploy (criterion 6).
- Android Studio open + USB device for APK install (criterion 6).

## -j 2 on every cargo build (M1 RAM rule).
