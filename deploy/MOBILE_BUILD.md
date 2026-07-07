# Ajna Mobile — Build & Deploy to Your Phone

## What's already done (verified on this machine)
- **MediaPipe model**: `samples/android/app/src/main/assets/face_landmarker.task` (3.75 MB).
- **iOS SDK**: `dist/AjnaSDK.xcframework` built (device + simulator slices) from the
  Rust core via `scripts/package_ios_xcframework.sh`. Verified: Rust core cross-compiles
  for `aarch64-apple-ios` + `aarch64-apple-ios-sim`; the iOS sample **typechecks clean**
  against the iOS SDK.
- **iOS app wired**: `ScanView.swift` now runs the real camera (`AjnaCameraAdapter`),
  Apple **Vision** liveness (`VNDetectFaceLandmarksRequest`) + OCR (`VNRecognizeTextRequest`),
  calls the Rust core FFI (`ajna_ui_config_validate`), and POSTs to the backend.
- **Android app wired** (source): `ScanActivity.kt` runs CameraX → MediaPipe blink
  liveness → ML Kit OCR → Ajna JNI (`AjnaSDK.kt`) → `ResultActivity`. Gradle deps in
  `samples/android/app/build.gradle.kts`.
- **Backend**: live locally at `http://localhost:8080/health` → `{"status":"ok",...}`.
- **Supabase**: migrated + seeded (`ajna_live_sk_demo_0000`).

## iOS — open in Xcode & deploy (needs your Mac/Apple ID)
1. Create the app project (one-time): Xcode → New → App → SwiftUI, name `AjnaVerifySample`,
   then add the `samples/ios/AjnaVerifySample/*.swift` + `platform/ios/AjnaCameraAdapter.swift`
   files, and set `Info.plist` `NSCameraUsageDescription`.
2. **Link the SDK**: drag `dist/AjnaSDK.xcframework` into the target → *Frameworks, Libraries,
   and Embedded Content* → Embed & Sign.
3. **Signing** (physical device): Target → Signing & Capabilities → check *Automatically
   manage signing* → select your Apple ID team. Connect your iPhone, select it as the run
   destination, press ⌘R. (Free Apple ID works for 7-day dev builds.)

## Android — open in Android Studio & deploy (needs the SDK/NDK — see below)
1. Build the AAR first (needs NDK — gated):
   `bash scripts/package_android_aar.sh` → `dist/AjnaSDK-0.1.0-release.aar`.
2. Open `samples/android/` in Android Studio (it will prompt to install the matching SDK).
3. Enable USB debugging on your phone, plug it in, press **Run ▶**. Gradle produces the APK
   and installs it.

## Local dev URL for your phone
Once `ngrok` is set up: `ngrok http 8080` → put the `https://…ngrok…` URL into the app's
Settings → Backend URL (iOS) / `Config.BACKEND_URL` (Android). Your phone then reaches the
local backend over the internet.
