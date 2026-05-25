// platform/android/BeamNativeBridge.kt
// JNI bridge from Kotlin to the native Beam SDK (tflite_bridge.cpp / libbeam_sdk.so).
// Declares all external JNI methods matching tflite_bridge.cpp exports.
// Provides Kotlin-friendly wrappers over the raw JNI surface.

package ai.surt.beam

import java.nio.ByteBuffer

/**
 * JNI bridge to the native Beam SDK.
 *
 * Lifecycle:
 *   1. Call [nativeCreateInferenceEngine] with the TFLite model path.
 *   2. Call [nativeCreateSession] to create a scan session.
 *   3. Call [nativeStartSession] with the current timestamp.
 *   4. For each camera frame: call [onFrame] (Kotlin-friendly wrapper).
 *   5. Poll [nativeGetState] until it returns COMPLETE (3) or FAILED (4).
 *   6. Call [nativeDestroySession] then [nativeDestroyInferenceEngine] to clean up.
 */
class BeamNativeBridge {

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

    // ─── Kotlin-friendly wrappers ────────────────────────────────────────────

    /**
     * Handle a frame delivered by [BeamCameraAdapter].
     * This is the primary integration point — wire [BeamCameraAdapter.delegate]
     * to call this method from [BeamCameraAdapter.processFrame].
     *
     * The [engineHandle] and [sessionHandle] must have been initialised before
     * the camera starts delivering frames.
     */
    fun onFrame(
        engineHandle:  Long,
        sessionHandle: Long,
        yBuffer:       ByteBuffer,
        uvBuffer:      ByteBuffer,
        width:         Int,
        height:        Int,
        yStride:       Int,
        uvStride:      Int,
        isNv12:        Boolean,
        timestampUs:   Long,
    ) {
        nativeOnFrame(
            engineHandle,
            sessionHandle,
            yBuffer,
            uvBuffer,
            width,
            height,
            yStride,
            uvStride,
            isNv12,
            timestampUs,
        )
    }

    /**
     * Convenience: read the [SessionState] enum from the raw integer state.
     */
    fun getSessionState(sessionHandle: Long): SessionState {
        return when (nativeGetState(sessionHandle)) {
            0 -> SessionState.IDLE
            1 -> SessionState.SCANNING
            2 -> SessionState.INFERRING
            3 -> SessionState.COMPLETE
            4 -> SessionState.FAILED
            else -> SessionState.FAILED
        }
    }

    /**
     * Camera error callback — invoked by [BeamCameraAdapter] on device failure.
     * Override or observe this in the host application.
     */
    open fun onCameraError(errorCode: Int) {
        android.util.Log.e("BeamNativeBridge", "Camera error: $errorCode")
    }

    companion object {
        init {
            // Load the shared native library. The .so is named libbeam_sdk.so.
            System.loadLibrary("beam_sdk")
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
