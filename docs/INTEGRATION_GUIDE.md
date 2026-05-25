# Beam SDK Integration Guide

## Overview

Beam SDK is a cross-platform document scanning library built by Surt AI.
This guide covers integrating Beam into Android, iOS, and Web (WASM) applications.

---

## Android Integration

### 1. Add the AAR to your project

Copy `BeamSDK-0.1.0-release.aar` into your app module's `libs/` directory, then add to `app/build.gradle`:

```groovy
dependencies {
    implementation fileTree(dir: 'libs', include: ['*.aar'])
}
```

### 2. Initialise in `Application.onCreate`

```kotlin
class MyApp : Application() {
    val bridge = BeamNativeBridge()
    var engineHandle = 0L
    var sessionHandle = 0L

    override fun onCreate() {
        super.onCreate()
        val modelPath = filesDir.absolutePath + "/beam_idv.tflite"
        engineHandle = bridge.nativeCreateInferenceEngine(modelPath)
    }
}
```

### 3. Start a scan session

```kotlin
val app = application as MyApp
app.sessionHandle = app.bridge.nativeCreateSession()
app.bridge.nativeStartSession(app.sessionHandle,
    System.nanoTime() / 1000L)

val cameraManager = getSystemService(CAMERA_SERVICE) as CameraManager
val adapter = BeamCameraAdapter(cameraManager, app.bridge)
adapter.open()
```

### 4. Handle the result

```kotlin
// Poll on a background thread (or use a callback wrapper)
while (app.bridge.getSessionState(app.sessionHandle) == SessionState.SCANNING) {
    Thread.sleep(40)
}
if (app.bridge.getSessionState(app.sessionHandle) == SessionState.COMPLETE) {
    // Result is available — retrieve via your result parcel mechanism
}
```

> **Note:** The `onFrame` callback is wired automatically by `BeamCameraAdapter` via `BeamNativeBridge.onFrame(...)`.

---

## iOS Integration

### 1. Add the XCFramework

Drag `BeamSDK.xcframework` into your Xcode project under **Frameworks, Libraries, and Embedded Content**. Set to **Embed & Sign**.

### 2. Scan a document

```swift
import BeamSDK

let scanner = BeamScanner()

do {
    try scanner.configure()
} catch {
    print("Camera unavailable: \(error)")
    return
}

scanner.startScan(config: BeamScanConfig(pqcSignResult: true)) { result in
    switch result {
    case .success(let scan):
        print("Type:", scan.documentType, "Country:", scan.issuingCountry)
        print("Confidence:", scan.confidence)
    case .failure(let error):
        print("Scan failed:", error)
    }
}
```

> All completion callbacks are dispatched on `DispatchQueue.main`.

---

## Web / WASM Integration

### 1. Install the npm package

```bash
npm install @surt/beam-sdk
```

### 2. Scan a document frame

```typescript
import BeamModule from '@surt/beam-sdk';

const Beam = await BeamModule();

// Load model into WASM filesystem at init time
const wasmSession = Beam.beam_wasm_create('/models/beam_idv.onnx');
const rustSession = Beam.beam_session_create({
    minQualityFrames: 3,
    timeoutMs: 30000,
    adaptiveGateLimit: 60,
    pqcSignResult: true,
    includeRawMrz: false,
});
Beam.beam_session_start(rustSession, BigInt(Date.now() * 1000));

// In your video frame loop:
function processFrame(imageData: ImageData) {
    // MANDATORY COPY: ImageData lives in JS heap; must be copied to WASM heap.
    // This is an expected cost — see WASM architecture note in SECURITY_MODEL.md.
    const ptr = Beam._malloc(imageData.width * imageData.height * 4);
    Beam.HEAPU8.set(imageData.data, ptr);
    Beam.beam_wasm_process_frame(
        wasmSession, ptr,
        imageData.width, imageData.height,
        rustSession
    );
    Beam._free(ptr);
}
```

> **WASM Copy Note:** The `ImageData.data → WASM heap` copy is **not a bug**. WASM cannot access browser memory directly. At 25fps / 1080p, this is a ~5 MB/s copy budget — negligible on modern hardware.

---

## Cleanup

### Android
```kotlin
bridge.nativeDestroySession(sessionHandle)
bridge.nativeDestroyInferenceEngine(engineHandle)
adapter.release()
```

### iOS
```swift
scanner.stopScan()
```

### WASM
```typescript
Beam.beam_wasm_destroy(wasmSession);
Beam.beam_session_destroy(rustSession);
```
