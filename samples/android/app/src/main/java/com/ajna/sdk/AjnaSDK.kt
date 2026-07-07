package com.ajna.sdk

import java.nio.ByteBuffer

/**
 * JNI bridge to the native Ajna SDK.
 *
 * Class name MUST be AjnaSDK to match JNI symbol prefix Java_ai_ajna_ajna_AjnaSDK_*
 * exported by tflite_bridge.cpp.
 *
 * Lifecycle:
 *   1. Call [initialize] with the TFLite model path.
 *   2. For each camera frame: call [onFrame] (handles are encapsulated).
 *   3. Poll [getSessionState] until it returns COMPLETE or FAILED.
 *   4. Call [destroy] to clean up.
 */
class AjnaSDK {

    // ─── Instance state (private) ────────────────────────────────────────────
    private var engineHandle: Long = 0L
    private var sessionHandle: Long = 0L

    // ─── Lifecycle methods ───────────────────────────────────────────────────

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

    // ─── Internal accessor (package-private for AjnaScanner) ─────────────────

    internal fun getSessionHandle(): Long = sessionHandle

    // ─── onFrame wrapper (NO handle parameters — uses instance state) ────────

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

    // ─── State query ─────────────────────────────────────────────────────────

    fun getSessionState(): SessionState = when (nativeGetState(sessionHandle)) {
        0 -> SessionState.IDLE
        1 -> SessionState.SCANNING
        2 -> SessionState.INFERRING
        3 -> SessionState.COMPLETE
        4 -> SessionState.FAILED
        else -> SessionState.FAILED
    }

    // ─── Camera error callback ───────────────────────────────────────────────

    open fun onCameraError(errorCode: Int) {
        android.util.Log.e("AjnaSDK", "Camera error: $errorCode")
    }

    // ─── Native JNI declarations ─────────────────────────────────────────────

    /**
     * Create a TFLite inference engine loaded with the model at [modelPath].
     * Returns an opaque native handle (> 0) on success, 0 on failure.
     */
    external fun nativeCreateInferenceEngine(modelPath: String): Long

    /**
     * Destroy the inference engine and free all associated native memory.
     * [handle] must be a value returned by [nativeCreateInferenceEngine].
     */
    external fun nativeDestroyInferenceEngine(handle: Long)

    /**
     * Create a new scan session.
     * Returns an opaque session handle (> 0). Caller must call [nativeDestroySession].
     */
    external fun nativeCreateSession(): Long

    /**
     * Destroy a scan session and free all associated native memory.
     */
    external fun nativeDestroySession(handle: Long)

    /**
     * Transition the session to Scanning state.
     * [timestampUs]: monotonic clock timestamp in microseconds.
     */
    external fun nativeStartSession(handle: Long, timestampUs: Long)

    /**
     * Process one YUV_420_888 frame.
     *
     * [engineHandle]: handle from [nativeCreateInferenceEngine].
     * [sessionHandle]: handle from [nativeCreateSession].
     * [yBuffer]: Y plane ByteBuffer (direct). Must not be retained after return.
     * [uvBuffer]: UV plane ByteBuffer (direct, NV12 interleaved or YV12 U plane).
     * [width], [height]: frame dimensions in pixels.
     * [yStride]: Y plane row stride in bytes (may be > width).
     * [uvStride]: UV plane row stride in bytes.
     * [isNv12]: true if pixel format is NV12 (planes[1].pixelStride == 2 from Camera2).
     * [timestampUs]: frame capture timestamp in microseconds.
     */
    external fun nativeOnFrame(
        engineHandle: Long,
        sessionHandle: Long,
        yBuffer:      ByteBuffer,
        uvBuffer:     ByteBuffer,
        width:        Int,
        height:       Int,
        yStride:      Int,
        uvStride:     Int,
        isNv12:       Boolean,
        timestampUs:  Long,
    )

    /**
     * Returns the current session state as an integer.
     * 0 = Idle, 1 = Scanning, 2 = Inferring, 3 = Complete, 4 = Failed.
     */
    external fun nativeGetState(sessionHandle: Long): Int

    // ─── Companion object ────────────────────────────────────────────────────

    companion object {
        init {
            System.loadLibrary("ajna_sdk")
        }
    }
}

/**
 * Mirror of the Rust SessionState enum for Kotlin consumers.
 */
enum class SessionState {
    IDLE,
    SCANNING,
    INFERRING,
    COMPLETE,
    FAILED,
}
