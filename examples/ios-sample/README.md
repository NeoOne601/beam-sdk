# Ajna iOS Sample (Swift / SwiftUI)

End-to-end demo: AVFoundation camera → `UiConfig` overlay → on-device liveness
(Vision face landmarks → `ajna-vision` FSM) → document scan → ML-DSA-65 signed
payload → POST to backend.

## What it wires together
- **AVFoundation** capture session (front camera liveness, back camera doc).
- **UiConfig overlay** — the declarative config the dashboard UI Customizer
  exports, validated by `ajna_ui_config_validate` before rendering.
- **ajna-core FFI** — the Rust core cross-compiled to a static lib, bridged via
  `ajna_ffi.h`; signs the parsed result with ML-DSA-65 at the edge.
- **Backend POST** — `/v1/nonce` then `/v1/verify` with the signed payload.

## Build (needs Xcode + a signing profile + a device — camera required)
1. Cross-compile the Rust core for iOS:
   ```bash
   rustup target add aarch64-apple-ios aarch64-apple-ios-sim
   (cd ../../core && cargo build --release --target aarch64-apple-ios -j 2)
   ```
   Use the repo's `scripts/package_ios_xcframework.sh` to produce
   `AjnaCore.xcframework`, then drag it into the Xcode project.
2. Add `include/ajna_ffi.h` to a bridging header.
3. Set `backendURL` / `apiKey` in `Config.swift`.
4. Run on a device (Signing & Capabilities → your team).

## Files
- `AjnaSample/ContentView.swift` — SwiftUI capture flow + overlay + submit.
- `AjnaSample/AjnaBridge.swift` — Swift wrapper over the C FFI.
