# Ajna Mobile — Build & Deploy to Your Phone

## Built & verified on this machine
| Artifact | Path | Status |
|---|---|---|
| MediaPipe model | `samples/android/app/src/main/assets/face_landmarker.task` | ✅ 3.75 MB |
| Android AAR (Rust core, 3 ABIs) | `dist/AjnaSDK-0.1.0-release.aar` | ✅ built via NDK |
| **Android APK** | `samples/android/app/build/outputs/apk/debug/app-debug.apk` | ✅ **64 MB, built** |
| iOS XCFramework | `dist/AjnaSDK.xcframework` | ✅ device + simulator |
| WASM core | `target/wasm32-unknown-emscripten/release/libajna_core.a` | ✅ compiles |
| Backend (local) | `http://localhost:8080/health` | ✅ live |
| Backend (public) | `https://ajna-verify.onrender.com/health` | ✅ live (prior deploy) |
| Supabase | migrated + seeded `ajna_live_sk_demo_0000` | ✅ |

The APK bundles `libajna_sdk.so` (our Rust core) for arm64-v8a / armeabi-v7a /
x86_64, plus MediaPipe + ML Kit native libs and the face-landmarker model.

## Reproduce the builds
```bash
# iOS SDK
bash scripts/package_ios_xcframework.sh          # → dist/AjnaSDK.xcframework

# Android SDK (set your NDK)
export ANDROID_NDK="/Volumes/Apple/Android/sdk/ndk/29.0.14206865"
bash scripts/package_android_aar.sh              # → dist/AjnaSDK-0.1.0-release.aar

# Android APK (uses external SDK + Android Studio's JBR)
export ANDROID_HOME="/Volumes/Apple/Android/sdk"
export JAVA_HOME="/Volumes/Apple/Applications/Android Studio.app/Contents/jbr/Contents/Home"
cd samples/android && ./gradlew :app:assembleDebug    # (or use the cached gradle 8.14)
```

## Android — install on your phone (adb, no Android Studio needed)
```bash
export ANDROID_HOME="/Volumes/Apple/Android/sdk"
"$ANDROID_HOME/platform-tools/adb" devices          # confirm your phone (USB debugging on)
"$ANDROID_HOME/platform-tools/adb" install -r \
  samples/android/app/build/outputs/apk/debug/app-debug.apk
```
Or open `samples/android/` in **Android Studio** → select your device → Run ▶.
Set the backend URL in Settings (defaults can point at `https://ajna-verify.onrender.com`).

## iOS — open in Xcode & deploy (needs your Apple ID for signing)
1. Xcode → New → App (SwiftUI), name `AjnaVerifySample`. Add the
   `samples/ios/AjnaVerifySample/*.swift` + `platform/ios/AjnaCameraAdapter.swift`
   files. Add `NSCameraUsageDescription` to Info.plist.
2. Drag `dist/AjnaSDK.xcframework` into the target → *Frameworks, Libraries, and
   Embedded Content* → **Embed & Sign**.
3. **Signing (device):** Target → Signing & Capabilities → *Automatically manage
   signing* → pick your Apple ID team. Connect iPhone, select it, ⌘R.
   (A free Apple ID gives 7-day dev builds.)

## Point the phone at your LOCAL backend (optional — ngrok)
The apps can use the live Render URL directly. To instead expose the local
`:8080` backend to your phone:
```bash
ngrok config add-authtoken <YOUR_NGROK_AUTHTOKEN>   # from dashboard.ngrok.com
ngrok http 8080                                      # copy the https://…ngrok URL
```
Put that URL into the app's backend setting.
