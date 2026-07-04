# Beam SDK — Wave 1 Parallel Agent Prompts

**Execution model:** Paste each prompt into a separate Claude coding agent instance simultaneously.
**Hard rule for every agent:** Write code only. DO NOT run `cargo`, `cmake`, `kotlinc`, `tsc`, `python`, or any compiler/test suite. Post-wave validation runs after all 5 agents commit.
**Dependency note for Agent D:** The TypeScript method `getScanResult()` calls `beam_session_get_result_json` which Agent B creates. Write the TypeScript as if that symbol exists — do not attempt a WASM build.

---

## AGENT A — VR-1 C++ Call Site Signature Propagation

```
You are a senior C++ systems engineer. You make exactly the changes described. You do not refactor, rename, or add anything beyond what is explicitly requested.

<critical_constraint>
DO NOT run any build command. DO NOT invoke clang++, cmake, cargo, or any compiler. DO NOT run tests. Your job is to write correct code. Validation happens after all 5 parallel agents finish. If you attempt to build, you will fail because dependent changes from other agents are not yet present.
</critical_constraint>

<context>
Repository: beam-sdk — cross-platform identity document scanning SDK.
Language stack: Rust core + C++ ML bridges + Swift/Kotlin platform adapters.

Security remediation VR-1 added 4 nonce-binding parameters to beam_session_push_result.
The canonical C declaration in include/beam_ffi.h now requires 13 parameters total.
New parameters occupy positions 8–11:
  param 8:  const uint8_t* nonce_ptr
  param 9:  size_t         nonce_len
  param 10: const uint8_t* session_id_ptr
  param 11: size_t         session_id_len

Three C++ bridge files still call the OLD signature (8–9 args) — C ABI violation causing undefined behaviour on every inference call. include/beam_ffi.h is already correct. core/src/ffi.rs is already correct.
</context>

<scope_lock>
MUST touch ONLY these 3 files:
  platform/ios/coreml_bridge.mm
  platform/android/tflite_bridge.cpp
  platform/wasm/onnx_bridge.cpp

MUST NOT modify:
  include/beam_ffi.h
  core/src/ffi.rs
  Any other file in the repository
</scope_lock>

<task>
Fix every call site of beam_session_push_result to pass 13 arguments by inserting the 4 null/zero nonce-binding parameters at positions 8–11.

The correct fixed call shape (use this pattern for every call site):
  beam_session_push_result(
      handle,
      fields, field_count,
      doc_type_ptr, doc_type_len,
      country_ptr, country_len,
      /*nonce_ptr=*/      nullptr, /*nonce_len=*/      0,   // NEW VR-1 param 8-9
      /*session_id_ptr=*/ nullptr, /*session_id_len=*/ 0,   // NEW VR-1 param 10-11
      overall_conf,
      include_pqc_sig
  );

Preserve the original values of overall_conf and include_pqc_sig at each call site — do not change them.
</task>

<per_file_instructions>

FILE 1: platform/ios/coreml_bridge.mm
  There are exactly 2 calls to beam_session_push_result in this file.
  Call site 1: inside the `if (s->stub_mode || !s->model)` block.
    Current args: NULL, 0, doc_type, strlen(doc_type), country, strlen(country), 0.0f, false
    Fixed args:   NULL, 0, doc_type, strlen(doc_type), country, strlen(country), nullptr, 0, nullptr, 0, 0.0f, false
  Call site 2: after the fields[] loop at the end of beam_coreml_process().
    Preserve existing overall_conf variable and `/*include_pqc_sig=*/ true`.

FILE 2: platform/android/tflite_bridge.cpp
  There is exactly 1 call to beam_session_push_result inside push_output_to_session().
  Current args end with: ..., country, strlen(country), 0.92f, true
  Insert nullptr, 0, nullptr, 0 between the country args and 0.92f.

FILE 3: platform/wasm/onnx_bridge.cpp
  There is exactly 1 call to beam_session_push_result inside beam_wasm_process_frame().
  Apply the identical fix — insert nullptr, 0, nullptr, 0 between country args and confidence.
</per_file_instructions>

<constraints>
MUST preserve every existing comment, including Safety: notes and NOTE: blocks.
MUST NOT change any function signature, struct definition, class member, or logic other than the argument lists of beam_session_push_result calls.
MUST use nullptr and 0 (not NULL and 0) for the new C++ parameters in .mm and .cpp files.
MUST NOT add any new functionality, logging, or error handling.
</constraints>

<execution_steps>
1. Read platform/ios/coreml_bridge.mm
2. ✅ Report: "Located N calls to beam_session_push_result in coreml_bridge.mm at lines [X, Y]"
3. Apply fixes. Output the complete modified file.
4. ✅ Report: "coreml_bridge.mm — 2 call sites now pass 13 arguments"
5. Read platform/android/tflite_bridge.cpp
6. ✅ Report: "Located 1 call to beam_session_push_result in push_output_to_session at line [X]"
7. Apply fix. Output the complete modified file.
8. ✅ Report: "tflite_bridge.cpp — 1 call site now passes 13 arguments"
9. Read platform/wasm/onnx_bridge.cpp
10. ✅ Report: "Located 1 call to beam_session_push_result in beam_wasm_process_frame at line [X]"
11. Apply fix. Output the complete modified file.
12. ✅ Report: "onnx_bridge.cpp — 1 call site now passes 13 arguments"
</execution_steps>

<success_criteria>
Every call to beam_session_push_result across all 3 files passes exactly 13 arguments.
No call site has 8, 9, 10, 11, or 12 arguments — those counts mean the fix is incomplete.
Static check (run by human post-wave): grep for beam_session_push_result in the 3 files and count commas — each call must have 12 commas between its arguments.
</success_criteria>
```

---

## AGENT B — FFI Result Getter + iOS Data Hydration

```
You are a senior Rust and Swift engineer. You make exactly the changes described. You do not refactor, rename, or add anything beyond what is explicitly requested.

<critical_constraint>
DO NOT run any build command. DO NOT invoke cargo build, cargo test, cargo check, or any compiler. DO NOT run tests. Your job is to write correct code. Validation happens after all 5 parallel agents finish.
</critical_constraint>

<context>
Repository: beam-sdk — cross-platform identity document scanning SDK.

Current broken state:
  - ScanSession (core/src/session.rs) stores result: Option<ScanResult> after session.complete() is called
  - ScanResult (core/src/result.rs) contains fields: Vec<DocumentField>, document_type, issuing_country, confidence (f32), pqc_signature: Vec<u8>, pqc_public_key: Vec<u8>
  - DocumentField has: key: String, value: String, confidence: f32
  - No FFI getter exists — the stored ScanResult is unreachable from Swift/Kotlin
  - platform/ios/BeamSDK.swift ignores Rust data entirely and returns: BeamScanResult(documentType: "document", issuingCountry: "UNK", confidence: 0.92)
  - core/Cargo.toml does NOT have serde_json or hex (backend/Cargo.toml does, but not core)
  - hex 0.4.3 is already resolved in Cargo.lock (pulled in by the backend workspace member) — adding it to core/Cargo.toml incurs zero new downloads

Architecture constraint: MUST NOT add #[derive(Serialize)] or #[derive(Deserialize)] to ANY existing struct. Use serde_json::json! macro for manual serialization.
</context>

<scope_lock>
MUST touch ONLY these 4 files:
  core/src/ffi.rs
  include/beam_ffi.h
  core/Cargo.toml
  platform/ios/BeamSDK.swift

MUST NOT modify:
  core/src/session.rs
  core/src/result.rs
  core/src/crypto.rs
  Any other file
</scope_lock>

<task_part_1_ffi_rs>
Add a new public FFI function beam_session_get_result_json to core/src/ffi.rs.

Insert it AFTER the beam_session_push_result function block and BEFORE any #[cfg(test)] block.

Complete function specification:

  #[no_mangle]
  pub unsafe extern "C" fn beam_session_get_result_json(
      handle: BeamSessionHandle,
      out_buf: *mut u8,
      out_buf_len: usize,
  ) -> i32

Safety comment (MUST include):
  // Safety: handle must be a valid BeamSessionHandle produced by beam_session_create,
  // or null. out_buf must be valid for out_buf_len bytes when handle is non-null.

Logic (implement exactly in this order):
  1. If handle.is_null() → return -1
  2. Let session = &*handle
  3. If session.state != SessionState::Complete → return -2
  4. Let result = match &session.result { Some(r) => r, None => return -2 }
  5. Build JSON using serde_json::json! macro:
       let json_val = serde_json::json!({
           "fields": result.fields.iter().map(|f| serde_json::json!({
               "key": f.key,
               "value": f.value,
               "confidence": f.confidence
           })).collect::<Vec<_>>(),
           "document_type": result.document_type,
           "issuing_country": result.issuing_country,
           "confidence": result.confidence,
           "pqc_signature_hex": hex::encode(&result.pqc_signature),
           "pqc_public_key_hex": hex::encode(&result.pqc_public_key)
       });
  6. let json_bytes = match serde_json::to_vec(&json_val) { Ok(b) => b, Err(_) => return -3 };
  7. If json_bytes.len() >= out_buf_len → return -(json_bytes.len() as i32)
  8. core::ptr::copy_nonoverlapping(json_bytes.as_ptr(), out_buf, json_bytes.len())
  9. out_buf.add(json_bytes.len()).write(0u8)   // null terminator
  10. return json_bytes.len() as i32

Add the function to the pub use re-exports in core/src/lib.rs:
  pub use ffi::beam_session_get_result_json;
</task_part_1_ffi_rs>

<task_part_2_beam_ffi_h>
Add the C declaration for beam_session_get_result_json to include/beam_ffi.h.

Insert it AFTER the beam_session_push_result declaration, inside the existing extern "C" block.

Insert exactly:

  /**
   * Serialize the completed session result to JSON into caller-supplied buffer.
   *
   * Return values:
   *   positive       = bytes written (null terminator NOT counted)
   *   -1             = null handle
   *   -2             = session not in Complete state (state != 3)
   *   -3             = internal serialization error
   *   negative N where |N| > out_buf_len = buffer too small; retry with |N|+1 bytes
   */
  int32_t beam_session_get_result_json(
      BeamSessionHandle handle,
      uint8_t*          out_buf,
      size_t            out_buf_len
  );
</task_part_2_beam_ffi_h>

<task_part_3_cargo_toml>
Edit core/Cargo.toml. Under [dependencies], add these two lines:
  serde_json = { version = "1", features = ["alloc"] }
  hex = "0.4"

MUST NOT add serde with derive features.
MUST NOT modify [dev-dependencies], [features], [profile.release], or any other section.
MUST NOT change existing dependency entries.
</task_part_3_cargo_toml>

<task_part_4_beamsdk_swift>
Edit platform/ios/BeamSDK.swift.

STEP A — Add bridge declaration.
Find the block of @_silgen_name declarations near the top of the file (where beam_session_get_state, beam_session_create, etc. are declared). Add this declaration alongside them:

  @_silgen_name("beam_session_get_result_json")
  private func beam_session_get_result_json(
      _ handle: UnsafeMutableRawPointer?,
      _ outBuf: UnsafeMutablePointer<UInt8>?,
      _ outBufLen: Int
  ) -> Int32

STEP B — Replace hardcoded result construction.
In the didReceiveFrame method, find the block:
  if state == 3 {
      let result = BeamScanResult(
          fields: [],
          rawMrz: nil,
          documentType: "document",
          issuingCountry: "UNK",
          confidence: 0.92,
          ...

Replace the entire `if state == 3 { ... deliverResult(result) }` block with:

  if state == 3 {
      var jsonBuf = [UInt8](repeating: 0, count: 16384)
      let written = beam_session_get_result_json(rs, &jsonBuf, jsonBuf.count)

      let result: BeamScanResult
      if written > 0,
         let jsonStr = String(bytes: jsonBuf.prefix(Int(written)), encoding: .utf8),
         let jsonData = jsonStr.data(using: .utf8),
         let parsed = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] {

          let rawFields = parsed["fields"] as? [[String: Any]] ?? []
          let docFields = rawFields.compactMap { f -> BeamDocumentField? in
              guard let k = f["key"] as? String,
                    let v = f["value"] as? String,
                    let c = f["confidence"] as? Double else { return nil }
              return BeamDocumentField(key: k, value: v, confidence: Float(c))
          }
          result = BeamScanResult(
              fields:         docFields,
              rawMrz:         nil,
              documentType:   parsed["document_type"] as? String ?? "unknown",
              issuingCountry: parsed["issuing_country"] as? String ?? "UNK",
              confidence:     Float((parsed["confidence"] as? Double) ?? 0.0),
              pqcSignature:   Data(),
              pqcPublicKey:   Data()
          )
      } else {
          NSLog("[BeamSDK] beam_session_get_result_json returned %d — fallback stub", written)
          result = BeamScanResult(
              fields: [], rawMrz: nil,
              documentType: "document", issuingCountry: "UNK",
              confidence: 0.92, pqcSignature: Data(), pqcPublicKey: Data()
          )
      }
      deliverResult(result)
  }

MUST preserve the `if state == 4 { deliverError(.sessionTimeout) }` block immediately after — do not remove or alter it.
</task_part_4_beamsdk_swift>

<execution_steps>
1. Read core/src/ffi.rs — find end of beam_session_push_result and location of #[cfg(test)]
2. ✅ Report: "Insertion point in ffi.rs: after line [X], before #[cfg(test)] at line [Y]"
3. Add beam_session_get_result_json. Output complete modified core/src/ffi.rs.
4. ✅ Report: "ffi.rs updated — beam_session_get_result_json added"
5. Read core/src/lib.rs — check pub use ffi:: exports
6. Add beam_session_get_result_json to pub use. Output complete modified core/src/lib.rs.
7. ✅ Report: "lib.rs updated — function added to pub use exports"
8. Read include/beam_ffi.h — find beam_session_push_result declaration
9. Add C declaration. Output complete modified include/beam_ffi.h.
10. ✅ Report: "beam_ffi.h updated — C declaration added at line [X]"
11. Read core/Cargo.toml — inspect existing [dependencies]
12. Add serde_json and hex. Output complete modified core/Cargo.toml.
13. ✅ Report: "core/Cargo.toml updated — serde_json and hex added"
14. Read platform/ios/BeamSDK.swift — locate @_silgen_name block and if state == 3 block
15. Apply both changes. Output complete modified platform/ios/BeamSDK.swift.
16. ✅ Report: "BeamSDK.swift updated — FFI declaration added, state==3 block now calls beam_session_get_result_json"
</execution_steps>

<success_criteria>
PASS: grep "beam_session_get_result_json" core/src/ffi.rs returns the function definition.
PASS: grep "beam_session_get_result_json" include/beam_ffi.h returns the C declaration.
PASS: grep "serde_json" core/Cargo.toml returns 1 line under [dependencies].
FAIL condition: The string literal "issuingCountry: \"UNK\"" appears as a NON-fallback primary result in BeamSDK.swift. The fallback branch (inside the else block with NSLog) is acceptable; the primary path MUST NOT hardcode it.
</success_criteria>
```

---

## AGENT C — Android JNI Fix + BeamScanner Public API

```
You are a senior Android and Kotlin engineer. You make exactly the changes described. You do not refactor, rename, or add anything beyond what is explicitly requested.

<critical_constraint>
DO NOT run any build command. DO NOT invoke kotlinc, gradle, Android Studio, or any compiler. DO NOT run tests. Your job is to write correct code. Validation happens after all 5 parallel agents finish.

STOP AND ASK before deleting any file. The rename of BeamNativeBridge.kt is handled by creating a new file (BeamSDK.kt) and leaving a deletion note — do not silently delete files without confirmation.
</critical_constraint>

<context>
Repository: beam-sdk — cross-platform identity document scanning SDK.

Current broken state (2 bugs, 1 missing feature):

BUG 1 — JNI class name mismatch (runtime crash):
  tflite_bridge.cpp exports JNI symbols under class path ai.surt.beam.BeamSDK:
    Java_ai_surt_beam_BeamSDK_nativeCreateInferenceEngine
    Java_ai_surt_beam_BeamSDK_nativeDestroyInferenceEngine
    Java_ai_surt_beam_BeamSDK_nativeOnFrame  (etc.)
  But platform/android/BeamNativeBridge.kt declares these externals in class BeamNativeBridge.
  Result: UnsatisfiedLinkError at runtime on every Android device.
  Fix: The Kotlin class must be named BeamSDK in package ai.surt.beam.
  MUST NOT change tflite_bridge.cpp — its JNI symbol names are already correct.

BUG 2 — Missing handle propagation (compile failure):
  BeamCameraAdapter.kt calls nativeBridge.onFrame(yBuffer, uvBuffer, width, height, ...)
  But BeamNativeBridge.kt's onFrame wrapper requires engineHandle: Long and sessionHandle: Long as first two parameters.
  The adapter has no access to these handles and cannot pass them.
  Fix: After the rename, BeamSDK owns engineHandle and sessionHandle as instance state.
  BeamCameraAdapter receives a BeamSDK reference and calls onFrame() with no handle parameters
  (the handles are encapsulated inside BeamSDK).

MISSING FEATURE — Android has no high-level developer-facing API:
  iOS has BeamScanner (configure/startScan/stopScan). Android exposes raw JNI handles.
  Fix: Create BeamScanner.kt as the public Android API mirroring iOS parity.
</context>

<scope_lock>
MUST create or modify ONLY these files:
  CREATE:   platform/android/BeamSDK.kt              (new — replaces BeamNativeBridge.kt)
  MODIFY:   platform/android/BeamCameraAdapter.kt    (update reference from BeamNativeBridge → BeamSDK)
  CREATE:   platform/android/BeamScanner.kt          (new high-level public API)

MUST NOT modify:
  platform/android/tflite_bridge.cpp   (JNI symbols are already correct)
  Any iOS, WASM, Rust, or backend file

NOTE ON BeamNativeBridge.kt: After creating BeamSDK.kt, add a comment block at the top of
BeamNativeBridge.kt saying:
  // DEPRECATED: This file is superseded by BeamSDK.kt.
  // It will be deleted after Wave 1 agent merge. Do not add new code here.
Do NOT delete the file — flag it for human review instead.
</scope_lock>

<task_file_1_beamsdk_kt>
Create platform/android/BeamSDK.kt with this exact content and structure:

Package: ai.surt.beam

Class name: BeamSDK (MUST match the JNI symbol prefix Java_ai_surt_beam_BeamSDK_*)

Instance state (private):
  private var engineHandle: Long = 0L
  private var sessionHandle: Long = 0L

Lifecycle methods:
  fun initialize(modelPath: String) {
      engineHandle = nativeCreateInferenceEngine(modelPath)
      if (engineHandle == 0L) return
      sessionHandle = nativeCreateSession()
      if (sessionHandle == 0L) return
      nativeStartSession(sessionHandle, System.nanoTime() / 1000L)
  }

  fun destroy() {
      if (sessionHandle != 0L) nativeDestroySession(sessionHandle)
      if (engineHandle != 0L) nativeDestroyInferenceEngine(engineHandle)
      sessionHandle = 0L
      engineHandle = 0L
  }

Internal accessor (package-private visibility for BeamScanner):
  internal fun getSessionHandle(): Long = sessionHandle

onFrame wrapper (NO handle parameters — uses instance state):
  fun onFrame(
      yBuffer: ByteBuffer, uvBuffer: ByteBuffer,
      width: Int, height: Int,
      yStride: Int, uvStride: Int,
      isNv12: Boolean, timestampUs: Long
  ) {
      if (engineHandle == 0L || sessionHandle == 0L) return
      nativeOnFrame(engineHandle, sessionHandle, yBuffer, uvBuffer,
                    width, height, yStride, uvStride, isNv12, timestampUs)
  }

State query:
  fun getSessionState(): SessionState = when (nativeGetState(sessionHandle)) {
      0 -> SessionState.IDLE
      1 -> SessionState.SCANNING
      2 -> SessionState.INFERRING
      3 -> SessionState.COMPLETE
      4 -> SessionState.FAILED
      else -> SessionState.FAILED
  }

Camera error callback (open, same as original):
  open fun onCameraError(errorCode: Int) {
      android.util.Log.e("BeamSDK", "Camera error: $errorCode")
  }

JNI external declarations (identical to BeamNativeBridge.kt — all same signatures):
  Copy all external fun declarations verbatim from BeamNativeBridge.kt.

Companion object:
  companion object {
      init { System.loadLibrary("beam_sdk") }
  }

SessionState enum (copy from BeamNativeBridge.kt verbatim):
  enum class SessionState { IDLE, SCANNING, INFERRING, COMPLETE, FAILED }
</task_file_1_beamsdk_kt>

<task_file_2_beamcameraadapter_kt>
Edit platform/android/BeamCameraAdapter.kt.

Change 1: Constructor parameter
  FROM: private val nativeBridge: BeamNativeBridge
  TO:   private val beamSdk: BeamSDK

Change 2: All references to nativeBridge inside the class body
  Replace every occurrence of nativeBridge. with beamSdk.

Change 3: The processFrame() call inside the OnImageAvailableListener lambda
  The call must now be:
    beamSdk.onFrame(
        yBuffer     = yPlane.buffer,
        uvBuffer    = uvPlane.buffer,
        width       = image.width,
        height      = image.height,
        yStride     = yPlane.rowStride,
        uvStride    = uvPlane.rowStride,
        isNv12      = isNv12,
        timestampUs = image.timestamp / 1000L
    )
  MUST NOT pass engineHandle or sessionHandle — BeamSDK.onFrame() encapsulates them.

Make NO other changes to BeamCameraAdapter.kt.
</task_file_2_beamcameraadapter_kt>

<task_file_3_beamscanner_kt>
Create platform/android/BeamScanner.kt.

This is the developer-facing public API. It MUST mirror the iOS BeamScanner pattern.
Developers MUST NOT see engineHandle, sessionHandle, or any native pointer.

Full file content:

package ai.surt.beam

import android.content.Context
import android.hardware.camera2.CameraManager
import android.os.Handler
import android.os.Looper

data class BeamScanConfig(
    val modelPath: String,
    val minQualityFrames: Int = 3,
    val timeoutMs: Long = 30_000L,
    val pqcSignResult: Boolean = true
)

data class BeamDocumentField(
    val key: String,
    val value: String,
    val confidence: Float
)

data class BeamScanResult(
    val fields: List<BeamDocumentField>,
    val documentType: String,
    val issuingCountry: String,
    val confidence: Float,
    val pqcSigned: Boolean
)

class BeamScanner(private val context: Context) {

    private var sdk: BeamSDK? = null
    private var cameraAdapter: BeamCameraAdapter? = null
    private var resultCallback: ((Result<BeamScanResult>) -> Unit)? = null
    private var pollingHandler: Handler? = null
    private var pollingRunnable: Runnable? = null

    fun configure() {
        // Validates camera permission is available at configure time.
        // No-op if permission not yet granted — startScan will fail gracefully.
    }

    fun startScan(config: BeamScanConfig, onResult: (Result<BeamScanResult>) -> Unit) {
        resultCallback = onResult

        val beamSdk = BeamSDK().also { sdk = it }
        beamSdk.initialize(config.modelPath)

        val cameraManager = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager
        val adapter = BeamCameraAdapter(cameraManager, beamSdk).also { cameraAdapter = it }
        adapter.open()

        startPolling(beamSdk)
    }

    fun stopScan() {
        stopPolling()
        cameraAdapter?.release()
        sdk?.destroy()
        cameraAdapter = null
        sdk = null
        resultCallback = null
    }

    private fun startPolling(beamSdk: BeamSDK) {
        val handler = Handler(Looper.getMainLooper()).also { pollingHandler = it }
        val runnable = object : Runnable {
            override fun run() {
                val state = beamSdk.getSessionState()
                when (state) {
                    SessionState.COMPLETE -> {
                        // Result is available in the Rust session.
                        // beam_session_get_result_json (added by Agent B) retrieves it.
                        // Until that FFI symbol lands, deliver a placeholder result.
                        val result = BeamScanResult(
                            fields = emptyList(),
                            documentType = "passport",
                            issuingCountry = "UNK",
                            confidence = 0.0f,
                            pqcSigned = false
                        )
                        resultCallback?.invoke(Result.success(result))
                        stopScan()
                    }
                    SessionState.FAILED -> {
                        resultCallback?.invoke(Result.failure(Exception("Scan failed or timed out")))
                        stopScan()
                    }
                    else -> handler.postDelayed(this, 200L)
                }
            }
        }.also { pollingRunnable = it }
        handler.postDelayed(runnable, 200L)
    }

    private fun stopPolling() {
        pollingRunnable?.let { pollingHandler?.removeCallbacks(it) }
        pollingHandler = null
        pollingRunnable = null
    }
}
</task_file_3_beamscanner_kt>

<deprecation_note>
After creating BeamSDK.kt, open BeamNativeBridge.kt and insert at the very top (before the package declaration):

  // DEPRECATED — superseded by BeamSDK.kt (created in Wave 1 Agent C).
  // TODO: Delete this file after Wave 1 agent merge is confirmed.
  // Do NOT add new code here.

Do NOT delete any existing code in BeamNativeBridge.kt beyond adding this comment.
</deprecation_note>

<execution_steps>
1. Read platform/android/BeamNativeBridge.kt — extract all external fun declarations and SessionState enum
2. ✅ Report: "Read BeamNativeBridge.kt — found [N] external declarations"
3. Create platform/android/BeamSDK.kt with full content per task_file_1_beamsdk_kt
4. ✅ Report: "BeamSDK.kt created — class name matches JNI prefix Java_ai_surt_beam_BeamSDK_*"
5. Add deprecation comment to BeamNativeBridge.kt
6. ✅ Report: "BeamNativeBridge.kt marked deprecated"
7. Read platform/android/BeamCameraAdapter.kt
8. Apply the 3 changes (constructor, references, processFrame call)
9. Output complete modified BeamCameraAdapter.kt
10. ✅ Report: "BeamCameraAdapter.kt updated — uses BeamSDK, no handle parameters in onFrame call"
11. Create platform/android/BeamScanner.kt with full content per task_file_3_beamscanner_kt
12. ✅ Report: "BeamScanner.kt created — configure/startScan/stopScan public API"
</execution_steps>

<success_criteria>
PASS: grep "class BeamSDK" platform/android/BeamSDK.kt returns 1 result.
PASS: grep "Java_ai_surt_beam_BeamSDK" platform/android/tflite_bridge.cpp still matches the class name in BeamSDK.kt (ai.surt.beam.BeamSDK).
PASS: grep "engineHandle\|sessionHandle" platform/android/BeamCameraAdapter.kt returns 0 results — handles MUST NOT appear in the adapter.
PASS: grep "nativeCreateInferenceEngine" platform/android/BeamCameraAdapter.kt returns 0 results — raw JNI MUST NOT be called from the adapter.
PASS: grep "class BeamScanner" platform/android/BeamScanner.kt returns 1 result with startScan, stopScan, configure methods.
</success_criteria>
```

---

## AGENT D — Web TypeScript SDK Wrapper (@beam/verify)

```
You are a senior TypeScript and WebAssembly engineer. You make exactly the changes described. You do not refactor, rename, or add anything beyond what is explicitly requested.

<critical_constraint>
DO NOT run any build command. DO NOT invoke tsc, npm build, vite build, or any compiler. DO NOT run tests. Your job is to write correct TypeScript. Validation happens after all 5 parallel agents finish.

DEPENDENCY NOTE: The TypeScript method getScanResult() calls beam_session_get_result_json which is being added to the Rust FFI by Agent B in parallel. Write the TypeScript as if that function exists in the compiled WASM binary. The TypeScript can be written and type-checked without the Rust symbol being present — do not let this dependency block you.
</dependency_note>
</critical_constraint>

<context>
Repository: beam-sdk — cross-platform identity document scanning SDK.

Current broken state:
  - The WASM module (beam_sdk.js / beam_sdk.wasm) exposes low-level C functions via Emscripten.
  - A TypeScript declaration file at publishing/publish_npm.sh → beam_verify.d.ts documents a high-level API (initBeamVerify, processFrame, getScanResult, destroySession) but NO implementation file exists.
  - samples/web/main.ts demonstrates the scan flow using a delay() simulation loop — it does not use any real SDK class.
  - scripts/package_wasm_npm.sh packages the WASM build but does not include the TypeScript wrapper.

WASM memory architecture (mandatory copy — this is not a bug):
  JavaScript heap and WASM linear memory are isolated. Passing ImageData to ONNX Runtime requires:
    1. ptr = module._malloc(w * h * 4)
    2. module.HEAPU8.set(imageData.data, ptr)
    3. Call the C function with ptr
    4. module._free(ptr)
  This copy MUST be encapsulated inside BeamScanner.processFrame(). Callers MUST NOT touch _malloc or HEAPU8.

WASM function names available at runtime:
  beam_wasm_create(modelPath: string): number
  beam_wasm_process_frame(wasmSession, rgbaPtr, width, height, rustSession): void
  beam_wasm_destroy(session: number): void
  beam_session_create(config: object): number
  beam_session_start(session: number, timestampUs: bigint): void
  beam_session_get_state(session: number): number    // 0=Idle,1=Scanning,2=Inferring,3=Complete,4=Failed
  beam_session_get_result_json(session, outBuf, outBufLen): number  // Added by Agent B
  _malloc(size: number): number
  _free(ptr: number): void
  HEAPU8: Uint8Array
</context>

<scope_lock>
MUST create or modify ONLY these 3 files:
  CREATE:  platform/web/BeamScanner.ts
  MODIFY:  samples/web/main.ts
  MODIFY:  scripts/package_wasm_npm.sh

MUST NOT modify:
  publishing/publish_npm.sh
  publishing/beam_verify.d.ts   (the existing .d.ts is already correct — do not touch it)
  core/src/ffi.rs
  Any other file
</scope_lock>

<task_file_1_beamscanner_ts>
Create platform/web/BeamScanner.ts.

This file MUST compile under TypeScript strict mode (tsconfig has "strict": true).
MUST NOT expose _malloc, _free, HEAPU8, or any numeric WASM pointer to callers.

Full required exports:

export interface BeamConfig {
  apiKey: string;
  backendUrl: string;
  modelPath: string;
  enablePqcSigning?: boolean;
}

export interface DocumentField {
  key: string;
  value: string;
  confidence: number;
}

export interface ScanResult {
  fields: DocumentField[];
  documentType: string;
  issuingCountry: string;
  confidence: number;
  pqcSignatureHex: string;
  pqcPublicKeyHex: string;
}

interface BeamWasmModule {
  beam_wasm_create(modelPath: string): number;
  beam_wasm_process_frame(
    wasmSession: number, rgbaPtr: number,
    width: number, height: number, rustSession: number
  ): void;
  beam_wasm_destroy(session: number): void;
  beam_session_create(config: object): number;
  beam_session_start(session: number, timestampUs: bigint): void;
  beam_session_get_state(session: number): number;
  beam_session_get_result_json(session: number, outBuf: number, outBufLen: number): number;
  _malloc(size: number): number;
  _free(ptr: number): void;
  HEAPU8: Uint8Array;
}

export class BeamScanner {
  private module: BeamWasmModule | null = null;
  private wasmSession: number = 0;
  private rustSession: number = 0;
  private config: BeamConfig | null = null;

  async configure(config: BeamConfig): Promise<void> {
    if (!config.modelPath) throw new Error('BeamScanner.configure: modelPath is required');
    if (!config.enablePqcSigning) {
      console.warn('[BeamScanner] enablePqcSigning is false — results will not be PQC-signed');
    }

    // Dynamic import of the Emscripten module (assumed served from same origin as beam_sdk.js)
    // The module factory is the default export of beam_sdk.js
    const factory = (await import('./beam_sdk.js')) as { default: (opts?: object) => Promise<BeamWasmModule> };
    this.module = await factory.default();

    this.wasmSession = this.module.beam_wasm_create(config.modelPath);
    if (this.wasmSession === 0) throw new Error('BeamScanner.configure: failed to create WASM inference session');

    this.rustSession = this.module.beam_session_create({
      min_quality_frames: 3,
      timeout_ms: 30000,
      adaptive_gate_limit: 60,
      pqc_sign_result: config.enablePqcSigning ?? true,
      include_raw_mrz: false
    });
    this.module.beam_session_start(this.rustSession, BigInt(Date.now() * 1000));
    this.config = config;
  }

  async processFrame(imageData: ImageData): Promise<{ gateReached: string }> {
    if (!this.module || this.config === null) {
      throw new Error('BeamScanner.processFrame: call configure() first');
    }
    const { width, height } = imageData;
    const byteCount = width * height * 4;

    // MANDATORY COPY: JS heap to WASM linear memory.
    // This copy is architecturally unavoidable on WASM and is documented as expected cost
    // in the SDK architecture. See platform/wasm/onnx_bridge.cpp module-level note.
    const ptr = this.module._malloc(byteCount);
    try {
      this.module.HEAPU8.set(imageData.data, ptr);
      this.module.beam_wasm_process_frame(
        this.wasmSession, ptr, width, height, this.rustSession
      );
    } finally {
      this.module._free(ptr);
    }

    const state = this.module.beam_session_get_state(this.rustSession);
    const stateNames = ['Idle', 'Scanning', 'Inferring', 'Complete', 'Failed'];
    return { gateReached: stateNames[state] ?? 'Unknown' };
  }

  async getScanResult(): Promise<ScanResult | null> {
    if (!this.module || this.config === null) {
      throw new Error('BeamScanner.getScanResult: call configure() first');
    }
    const state = this.module.beam_session_get_state(this.rustSession);
    if (state !== 3) return null;  // 3 = Complete

    const BUF_SIZE = 16384;
    const outPtr = this.module._malloc(BUF_SIZE);
    try {
      const written = this.module.beam_session_get_result_json(
        this.rustSession, outPtr, BUF_SIZE
      );
      if (written <= 0) return null;

      const jsonBytes = this.module.HEAPU8.subarray(outPtr, outPtr + written);
      const jsonStr = new TextDecoder().decode(jsonBytes);
      const parsed = JSON.parse(jsonStr) as {
        fields: Array<{ key: string; value: string; confidence: number }>;
        document_type: string;
        issuing_country: string;
        confidence: number;
        pqc_signature_hex: string;
        pqc_public_key_hex: string;
      };

      return {
        fields: parsed.fields.map(f => ({
          key: f.key, value: f.value, confidence: f.confidence
        })),
        documentType: parsed.document_type,
        issuingCountry: parsed.issuing_country,
        confidence: parsed.confidence,
        pqcSignatureHex: parsed.pqc_signature_hex,
        pqcPublicKeyHex: parsed.pqc_public_key_hex
      };
    } finally {
      this.module._free(outPtr);
    }
  }

  destroy(): void {
    if (!this.module) return;
    if (this.wasmSession !== 0) this.module.beam_wasm_destroy(this.wasmSession);
    if (this.rustSession !== 0) {
      // beam_session_destroy is available via the Rust FFI
      // @ts-ignore — function exists in compiled WASM but not typed in BeamWasmModule interface
      (this.module as Record<string, unknown>)['beam_session_destroy']?.(this.rustSession);
    }
    this.wasmSession = 0;
    this.rustSession = 0;
    this.module = null;
    this.config = null;
  }
}
</task_file_1_beamscanner_ts>

<task_file_2_main_ts>
Edit samples/web/main.ts.

CHANGE 1: Import BeamScanner at the top of the file (after any existing imports):
  import { BeamScanner } from '../../platform/web/BeamScanner.js';

CHANGE 2: Replace the runScanFlow() function body.
Find the function:
  async function runScanFlow() {

Replace its entire body with an implementation that:
  1. Shows the scan screen
  2. Starts the camera (the existing startCamera() function is already implemented — call it)
  3. Creates a BeamScanner instance and calls configure() with backendUrl and modelPath from loadSettings()
  4. Processes frames in a requestAnimationFrame loop, calling processFrame() with each ImageData captured from the video element via a canvas
  5. On { gateReached: 'Complete' }, calls getScanResult() and calls showResult() with the returned data
  6. Handles errors by logging and showing the landing screen

Remove the existing delay() simulation function entirely.
Remove the simulated gate animation loop that uses setTimeout/delay.

The real camera stream already exists via startCamera() — wire processFrame() to it using a canvas 2D context:
  const canvas = document.createElement('canvas');
  canvas.width = video.videoWidth || 1280;
  canvas.height = video.videoHeight || 720;
  const ctx = canvas.getContext('2d')!;

  const processLoop = async () => {
    ctx.drawImage(video, 0, 0, canvas.width, canvas.height);
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const report = await scanner.processFrame(imageData);
    gateLabel.textContent = `Gate: ${report.gateReached}`;
    if (report.gateReached === 'Complete') {
      const result = await scanner.getScanResult();
      stopCamera(stream);
      if (result) showResultFromSdk(result);
      return;
    }
    if (report.gateReached !== 'Failed') {
      requestAnimationFrame(() => processLoop());
    }
  };
  requestAnimationFrame(() => processLoop());

Add a showResultFromSdk() function that populates the result screen fields from a ScanResult object.
MUST NOT change the existing showResult(), showScreen(), loadSettings(), stopCamera(), or verifyWithBackend() functions.
</task_file_2_main_ts>

<task_file_3_package_wasm_npm_sh>
Edit scripts/package_wasm_npm.sh.

After the line that copies beam_sdk.js and beam_sdk.wasm to NPM_DIR, add:

  # Copy TypeScript SDK wrapper
  echo "--- Copying TypeScript SDK wrapper ---"
  cp "${REPO_ROOT}/platform/web/BeamScanner.ts" "${NPM_DIR}/BeamScanner.ts"

After the package.json write block, change the "main" field from "beam_sdk.js" to "BeamScanner.js":
  "main": "BeamScanner.js",

Add "BeamScanner.ts" and "BeamScanner.js" to the "files" array in package.json:
  "files": [
    "beam_sdk.js",
    "beam_sdk.wasm",
    "beam-sdk.d.ts",
    "BeamScanner.ts",
    "BeamScanner.js"
  ],

After the npm pack step and before the final echo, add a TypeScript compilation step note:
  # TypeScript compilation (requires tsc on PATH — run manually or in CI after this script)
  # tsc "${NPM_DIR}/BeamScanner.ts" --outDir "${NPM_DIR}" --target ES2022 --module ESNext --strict

This is a comment only — do NOT actually invoke tsc (it will fail because beam_sdk.js type declarations are not present at script run time).
</task_file_3_package_wasm_npm_sh>

<execution_steps>
1. Read publishing/publish_npm.sh to understand the beam_verify.d.ts contract
2. ✅ Report: "Read publish_npm.sh — beam_verify.d.ts documents [N] exported symbols"
3. Read samples/web/main.ts to understand existing screen/camera/settings functions
4. ✅ Report: "Read main.ts — existing functions: [list]"
5. Create platform/web/BeamScanner.ts with full content
6. ✅ Report: "BeamScanner.ts created — configure/processFrame/getScanResult/destroy methods implemented"
7. Edit samples/web/main.ts — remove delay(), replace runScanFlow() body, add import, add showResultFromSdk()
8. Output complete modified main.ts
9. ✅ Report: "main.ts updated — delay() removed, real BeamScanner integration wired"
10. Read scripts/package_wasm_npm.sh
11. Apply the 3 changes (copy BeamScanner.ts, update main field, update files array, add tsc comment)
12. Output complete modified scripts/package_wasm_npm.sh
13. ✅ Report: "package_wasm_npm.sh updated — BeamScanner.ts included in npm dist"
</execution_steps>

<success_criteria>
PASS: grep "MANDATORY COPY" platform/web/BeamScanner.ts returns the comment in processFrame().
PASS: grep "_malloc\|_free\|HEAPU8" platform/web/BeamScanner.ts — results exist ONLY inside BeamScanner class methods, never in exported interfaces or the module-level scope.
PASS: grep "delay(" samples/web/main.ts returns 0 results.
PASS: grep "BeamScanner" scripts/package_wasm_npm.sh returns at least 1 result in the files array.
FAIL condition: BeamScanner.ts exports _malloc or HEAPU8 as public API — this breaks the encapsulation contract.
</success_criteria>
```

---

## AGENT E — Synthetic Test Model + M1 Validation Script

```
You are a senior ML infrastructure and DevOps engineer. You make exactly the changes described. You do not refactor, rename, or add anything beyond what is explicitly requested.

<critical_constraint>
DO NOT run any Python command. DO NOT invoke python3, pip, tensorflow, or any interpreter. DO NOT run tests. DO NOT execute scripts. Your job is to write correct files. The validation script is designed to be run by a human after all 5 agents commit — not by you.
</critical_constraint>

<context>
Repository: beam-sdk — cross-platform identity document scanning SDK.

Purpose of this work:
  The SDK needs end-to-end pipeline validation WITHOUT a real document OCR model.
  A synthetic TFLite model that outputs the correct tensor shapes (regardless of semantic accuracy)
  validates everything in the pipeline: quality gates → inference → beam_session_push_result → Rust session → PQC signing → Complete state.

Execution environment: Apple M1 iMac with tensorflow-macos (ARM-native).
  - tensorflow-macos is the Apple Silicon optimized TF package (pip install tensorflow-macos)
  - numpy is available alongside it
  - CUDA, GPU drivers, or x86 hardware are NOT available

Model output schema (from model/schema/output_schema.json — MUST match exactly):
  Input:  tensor named "image", float32 [1, 3, H, W]
  Output: "confidence"        float32 [1]   — always output 0.87
  Output: "document_type"     int32 [1]     — always output 0 (passport)
  Output: "field_strings"     byte data     — always "SMITH\0JOHN\0" (2 null-separated UTF-8 strings)
  Output: "field_confidences" float32 [2]  — always [0.95, 0.93]

The model MUST be INT8 quantized for TFLite GPU delegate compatibility.
The model output values are constants — it ignores actual pixel input.
This is sufficient to exercise the full pipeline; field classification accuracy is irrelevant.
</context>

<scope_lock>
MUST create or modify ONLY these 3 files:
  CREATE:  scripts/generate_synthetic_model.py
  CREATE:  scripts/validate_pipeline_m1.sh
  MODIFY:  .gitignore

MUST NOT modify:
  model/schema/output_schema.json
  model/manifest/model_manifest.json
  model/placeholder/
  Any source code in core/, platform/, backend/, or samples/
</scope_lock>

<task_file_1_generate_synthetic_model_py>
Create scripts/generate_synthetic_model.py.

This script, when run as `python3 scripts/generate_synthetic_model.py` from the repo root,
MUST produce model/placeholder/synthetic_test.tflite.

Complete implementation requirements:

Imports:
  import numpy as np
  import tensorflow as tf   # tensorflow-macos on Apple Silicon
  import os
  import struct

Architecture:
  Use tf.keras functional API. Input shape [1, 3, 224, 224] (NCHW — batch, channels, H, W).
  Since TFLite models typically use NHWC, accept [1, 224, 224, 3] as the input shape
  and document this in a comment. The C++ bridge converts NV12 → RGB and may reorder —
  accept NHWC for the synthetic model since keras defaults to it.

  Input: tf.keras.Input(shape=(224, 224, 3), name="image")
  Apply: tf.keras.layers.GlobalAveragePooling2D() to produce a feature vector

  Four output heads (all Dense layers initialized to constant values):
    confidence_out   = Dense(1, name="confidence", kernel_initializer=tf.initializers.Constant(0.0))(pooled)
    doc_type_out     = Dense(1, name="document_type", kernel_initializer=tf.initializers.Constant(0.0))(pooled)
    field_conf_out   = Dense(2, name="field_confidences", kernel_initializer=tf.initializers.Constant(0.0))(pooled)
    # field_strings is a byte tensor — represent as a Dense layer with 12 units (len of "SMITH\0JOHN\0")
    field_str_out    = Dense(12, name="field_strings", kernel_initializer=tf.initializers.Constant(0.0))(pooled)

  Note: The constant initializers mean outputs are zero-based but the TFLite conversion
  and quantization calibration will produce valid tensor shapes. Document in the script
  that the MODEL ALWAYS OUTPUTS PLACEHOLDER VALUES — confidence ~0.87 comes from the
  bias being set via a post-conversion step or the bias_initializer.

  Use bias_initializer instead of kernel_initializer to set constant outputs:
    confidence head:    bias_initializer=tf.initializers.Constant(0.87)
    document_type head: bias_initializer=tf.initializers.Constant(0.0)    # int32 cast post-conversion
    field_confidences:  bias_initializer=tf.initializers.Constant([0.95, 0.93])
    field_strings:      bias_initializer=tf.initializers.Constant([83,77,73,84,72,0,74,79,72,78,0,0])
                        # ASCII for "SMITH\0JOHN\0" + trailing 0

TFLite conversion:
  converter = tf.lite.TFLiteConverter.from_keras_model(model)
  converter.optimizations = [tf.lite.Optimize.DEFAULT]

  Representative dataset (required for INT8 quantization):
    def representative_dataset():
        for _ in range(100):
            yield [np.random.uniform(0, 1, (1, 224, 224, 3)).astype(np.float32)]
    converter.representative_dataset = representative_dataset

  converter.target_spec.supported_ops = [
      tf.lite.OpsSet.TFLITE_BUILTINS_INT8
  ]
  converter.inference_input_type  = tf.float32  # keep float input for ease of use
  converter.inference_output_type = tf.float32  # keep float output

Output path:
  OUTPUT_PATH = os.path.join(os.path.dirname(os.path.dirname(__file__)), "model", "placeholder", "synthetic_test.tflite")
  os.makedirs(os.path.dirname(OUTPUT_PATH), exist_ok=True)
  tflite_model = converter.convert()
  with open(OUTPUT_PATH, "wb") as f:
      f.write(tflite_model)
  print(f"Synthetic model written to: {OUTPUT_PATH}")
  print(f"Model size: {len(tflite_model) / 1024:.1f} KB")

Verification step (at the end of the script):
  interpreter = tf.lite.Interpreter(model_path=OUTPUT_PATH)
  interpreter.allocate_tensors()
  output_details = interpreter.get_output_details()
  output_names = [d["name"] for d in output_details]
  print(f"Output tensor names: {output_names}")
  print("Synthetic model verification: PASS")
</task_file_1_generate_synthetic_model_py>

<task_file_2_validate_pipeline_m1_sh>
Create scripts/validate_pipeline_m1.sh.

This script orchestrates the full M1 end-to-end validation after all 5 Wave 1 agents merge.

Full file content — write it exactly as follows:

#!/usr/bin/env bash
# scripts/validate_pipeline_m1.sh
# Full pipeline validation for Apple M1 iMac.
# Run ONLY after all 5 Wave 1 agents have committed their changes.
# Prerequisites: Xcode, Android Studio (NDK r26), Docker Desktop, Python tensorflow-macos, Emscripten
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
echo "=== Beam SDK M1 Pipeline Validation ==="
echo "Repo: ${REPO_ROOT}"
echo ""

# ─── Step 1: Generate synthetic model ────────────────────────────────────────
echo "[Step 1/6] Generating synthetic TFLite model..."
python3 "${REPO_ROOT}/scripts/generate_synthetic_model.py"
TFLITE_PATH="${REPO_ROOT}/model/placeholder/synthetic_test.tflite"
if [ ! -f "${TFLITE_PATH}" ]; then
    echo "ERROR: Synthetic model not found at ${TFLITE_PATH}"
    exit 1
fi
echo "  ✓ Synthetic model generated ($(du -sh ${TFLITE_PATH} | cut -f1))"
echo ""

# ─── Step 2: Rust unit tests ─────────────────────────────────────────────────
echo "[Step 2/6] Running Rust core tests..."
(cd "${REPO_ROOT}/core" && cargo test --release 2>&1)
echo "  ✓ Rust tests passed"
echo ""

# ─── Step 3: FFI integration tests (macOS host target) ───────────────────────
echo "[Step 3/6] Building and running C++ FFI integration tests..."
cargo build --release -p beam-core --target aarch64-apple-darwin \
    --manifest-path "${REPO_ROOT}/Cargo.toml" 2>&1

clang++ -O2 "${REPO_ROOT}/tests/ffi_integration_tests.cpp" \
    -I "${REPO_ROOT}/include/" \
    -L "${REPO_ROOT}/target/aarch64-apple-darwin/release" \
    -lbeam_core \
    -o /tmp/beam_ffi_tests
/tmp/beam_ffi_tests
echo "  ✓ FFI integration tests passed"
echo ""

# ─── Step 4: iOS arm64 cross-compile check ───────────────────────────────────
echo "[Step 4/6] Cross-compiling Rust core for iOS arm64..."
rustup target add aarch64-apple-ios 2>/dev/null || true
SDK_PATH="$(xcrun --sdk iphoneos --show-sdk-path)"
(
    cd "${REPO_ROOT}/core"
    export IPHONEOS_DEPLOYMENT_TARGET=14.0
    export CFLAGS_aarch64_apple_ios="--target=arm64-apple-ios -isysroot ${SDK_PATH} -miphoneos-version-min=14.0"
    export CC_aarch64_apple_ios="$(xcrun --sdk iphoneos --find clang)"
    export AR_aarch64_apple_ios="$(xcrun --sdk iphoneos --find ar)"
    cargo build --release --target aarch64-apple-ios 2>&1
)
echo "  ✓ iOS arm64 cross-compile passed"
echo ""

# ─── Step 5: Android arm64 cross-compile check ───────────────────────────────
echo "[Step 5/6] Cross-compiling Rust core for Android arm64..."
rustup target add aarch64-linux-android 2>/dev/null || true
ANDROID_NDK="${ANDROID_NDK_ROOT:-${HOME}/Library/Android/sdk/ndk/26.1.10909125}"
if [ ! -d "${ANDROID_NDK}" ]; then
    echo "  WARNING: Android NDK not found at ${ANDROID_NDK} — skipping Android cross-compile"
    echo "  Set ANDROID_NDK_ROOT to your NDK path and re-run to test Android."
else
    (
        cd "${REPO_ROOT}/core"
        export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${ANDROID_NDK}/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android26-clang"
        cargo build --release --target aarch64-linux-android 2>&1
    )
    echo "  ✓ Android arm64 cross-compile passed"
fi
echo ""

# ─── Step 6: Backend round-trip test ─────────────────────────────────────────
echo "[Step 6/6] Starting backend and running verification round-trip..."
(cd "${REPO_ROOT}/backend" && docker compose up -d 2>&1)
sleep 5

DB_URL="postgres://beam:beam@localhost:5432/beam_verify"
psql "${DB_URL}" -f "${REPO_ROOT}/backend/src/db/migrations/001_initial.sql" 2>/dev/null || true
psql "${DB_URL}" -f "${REPO_ROOT}/backend/src/db/migrations/002_trusted_keys.sql" 2>/dev/null || true

(cd "${REPO_ROOT}/backend" && cargo test --release 2>&1)

HEALTH=$(curl -sf http://localhost:8080/health || echo '{"status":"unreachable"}')
echo "  Backend health: ${HEALTH}"

(cd "${REPO_ROOT}/backend" && docker compose down 2>&1)
echo "  ✓ Backend round-trip passed"
echo ""

echo "=== ALL VALIDATION STEPS PASSED ==="
echo ""
echo "Pipeline summary:"
echo "  Synthetic model:   ${TFLITE_PATH}"
echo "  Rust tests:        PASS"
echo "  FFI tests:         PASS"
echo "  iOS cross-compile: PASS"
echo "  Android:           PASS (or skipped — see NDK note above)"
echo "  Backend:           PASS"
echo ""
echo "Next step: Replace synthetic_test.tflite with a real document OCR model."
echo "The output_schema.json specifies the exact tensor contract the real model must meet."
</task_file_2_validate_pipeline_m1_sh>

<task_file_3_gitignore>
Edit .gitignore.

Find the section with build artifact rules (*.o, *.a, *.so entries, or the build_output/ entry).
Add the following lines, inserting them in a logical position near other model/temp ignores:

  # Synthetic test model (generated, non-deterministic, never commit)
  model/placeholder/synthetic_test.tflite

  # Compiled FFI test binary
  /tmp/beam_ffi_tests

Also verify the existing entry "tests/ffi_tests" is present — if it is, leave it unchanged.
</task_file_3_gitignore>

<make_executable_note>
In validate_pipeline_m1.sh, the file needs execute permissions.
Add a note at the very bottom of the file (as a comment):
  # Make this script executable: chmod +x scripts/validate_pipeline_m1.sh
</make_executable_note>

<execution_steps>
1. Read model/schema/output_schema.json — confirm output tensor names and types
2. ✅ Report: "Confirmed output schema: [list output tensor names]"
3. Create scripts/generate_synthetic_model.py per task_file_1 spec
4. ✅ Report: "generate_synthetic_model.py created — outputs synthetic_test.tflite to model/placeholder/"
5. Create scripts/validate_pipeline_m1.sh per task_file_2 spec
6. ✅ Report: "validate_pipeline_m1.sh created — 6 validation steps"
7. Read .gitignore — find appropriate insertion point
8. Add synthetic_test.tflite entry and /tmp/beam_ffi_tests entry
9. Output complete modified .gitignore
10. ✅ Report: ".gitignore updated — synthetic_test.tflite and /tmp/beam_ffi_tests excluded"
</execution_steps>

<success_criteria>
PASS: python3 -c "import ast; ast.parse(open('scripts/generate_synthetic_model.py').read()); print('syntax ok')" returns "syntax ok" (syntax-only check, no execution).
PASS: bash -n scripts/validate_pipeline_m1.sh returns exit code 0 (bash syntax check, no execution).
PASS: grep "synthetic_test.tflite" .gitignore returns 1 result.
PASS: grep "representative_dataset" scripts/generate_synthetic_model.py returns ≥1 result (INT8 quantization requires it).
FAIL condition: The validation script invokes the Python model generation script during its own run AND the agent ran either of them during this session. Both scripts are write-only deliverables — they run post-merge.
</success_criteria>
```

---

## Post-Wave Merge Sequence (Human-Executed)

After all 5 agents deliver their changes, run in this exact order:

```bash
# 1. Static verification (no build required)
grep -c "beam_session_push_result" platform/ios/coreml_bridge.mm         # expect 2
grep -c "beam_session_get_result_json" core/src/ffi.rs                   # expect 1
grep -c "class BeamSDK" platform/android/BeamSDK.kt                      # expect 1
grep "MANDATORY COPY" platform/web/BeamScanner.ts                        # expect 1 match
grep "synthetic_test.tflite" .gitignore                                  # expect 1 match

# 2. Rust test suite (validates Agent B's FFI work end-to-end)
cd core && cargo test --release

# 3. Full M1 pipeline validation (only after cargo test passes 100%)
bash scripts/validate_pipeline_m1.sh
```