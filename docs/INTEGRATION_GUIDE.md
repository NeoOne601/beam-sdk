# Ajna SDK Integration Guide

## Overview

Ajna SDK is a cross-platform document scanning library built by Ajna AI.
This guide covers integrating Ajna into Android, iOS, and Web (WASM) applications.

---

## Android Integration

### 1. Add the AAR to your project

Copy `AjnaSDK-0.1.0-release.aar` into your app module's `libs/` directory, then add to `app/build.gradle`:

```groovy
dependencies {
    implementation fileTree(dir: 'libs', include: ['*.aar'])
}
```

### 2. Initialise in `Application.onCreate`

```kotlin
class MyApp : Application() {
    val bridge = AjnaNativeBridge()
    var engineHandle = 0L
    var sessionHandle = 0L

    override fun onCreate() {
        super.onCreate()
        val modelPath = filesDir.absolutePath + "/ajna_idv.tflite"
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
val adapter = AjnaCameraAdapter(cameraManager, app.bridge)
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

> **Note:** The `onFrame` callback is wired automatically by `AjnaCameraAdapter` via `AjnaNativeBridge.onFrame(...)`.

---

## iOS Integration

### 1. Add the XCFramework

Drag `AjnaSDK.xcframework` into your Xcode project under **Frameworks, Libraries, and Embedded Content**. Set to **Embed & Sign**.

### 2. Scan a document

```swift
import AjnaSDK

let scanner = AjnaScanner()

do {
    try scanner.configure()
} catch {
    print("Camera unavailable: \(error)")
    return
}

scanner.startScan(config: AjnaScanConfig(pqcSignResult: true)) { result in
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
npm install @ajna/sdk
```

### 2. Scan a document frame

```typescript
import AjnaModule from '@ajna/sdk';

const Ajna = await AjnaModule();

// Load model into WASM filesystem at init time
const wasmSession = Ajna.ajna_wasm_create('/models/ajna_idv.onnx');
const rustSession = Ajna.ajna_session_create({
    minQualityFrames: 3,
    timeoutMs: 30000,
    adaptiveGateLimit: 60,
    pqcSignResult: true,
    includeRawMrz: false,
});
Ajna.ajna_session_start(rustSession, BigInt(Date.now() * 1000));

// In your video frame loop:
function processFrame(imageData: ImageData) {
    // MANDATORY COPY: ImageData lives in JS heap; must be copied to WASM heap.
    // This is an expected cost — see WASM architecture note in SECURITY_MODEL.md.
    const ptr = Ajna._malloc(imageData.width * imageData.height * 4);
    Ajna.HEAPU8.set(imageData.data, ptr);
    Ajna.ajna_wasm_process_frame(
        wasmSession, ptr,
        imageData.width, imageData.height,
        rustSession
    );
    Ajna._free(ptr);
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
Ajna.ajna_wasm_destroy(wasmSession);
Ajna.ajna_session_destroy(rustSession);
```
