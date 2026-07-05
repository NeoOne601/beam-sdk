# Ajna Android Sample (Kotlin / Jetpack Compose)

End-to-end demo: CameraX preview → `UiConfig` overlay → on-device MediaPipe
liveness FSM → document scan → ML-DSA-65 signed payload → POST to backend.

## What it wires together
- **CameraX** front camera for liveness, back camera for document capture.
- **UiConfig overlay** — reads the same declarative config the dashboard UI
  Customizer exports (`ajna_ui_config_validate`) and draws the capture frame.
- **MediaPipe FaceMesh** → `FaceLandmarks.observe(...)` → the `ajna-vision`
  liveness FSM (blink / turn / smile).
- **ajna-core JNI** — the Rust core compiled to `libajna_core.so`, signing the
  scan result with ML-DSA-65 at the edge.
- **Backend POST** — `/v1/nonce` then `/v1/verify` with the signed payload.

## Build (needs Android SDK + NDK — not runnable from CI)
1. Build the Rust core for Android ABIs (produces `libajna_core.so`):
   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi
   export ANDROID_NDK=/path/to/ndk
   (cd ../../core && cargo build --release --target aarch64-linux-android -j 2)
   ```
   Drop the `.so` into `app/src/main/jniLibs/<abi>/`.
2. Add MediaPipe: `com.google.mediapipe:tasks-vision` (Face Landmarker) and
   place `face_landmarker.task` in `app/src/main/assets/`.
3. Set `AJNA_BACKEND_URL` in `local.properties` or `Config.kt`.
4. `./gradlew installDebug` onto a device (camera required — no emulator).

## Files
- `app/src/main/java/com/ajna/sample/MainActivity.kt` — Compose UI + pipeline.
- `app/src/main/java/com/ajna/sample/AjnaNative.kt` — JNI declarations.
- `app/build.gradle.kts` — dependencies.
