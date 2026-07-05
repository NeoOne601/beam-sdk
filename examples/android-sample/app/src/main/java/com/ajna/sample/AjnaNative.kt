package com.ajna.sample

/**
 * JNI seam to the Rust `ajna-core` static/shared library (`libajna_core.so`).
 *
 * These `external` functions map to the `#[no_mangle]` exports in
 * `core/src/ffi.rs`. The Rust side owns quality gates, the session state
 * machine, ML-DSA-65 signing, and canonical-bytes encoding; Kotlin only
 * marshals camera frames and pulls the signed JSON result back out.
 */
object AjnaNative {
    init {
        System.loadLibrary("ajna_core")
    }

    /** Validate a declarative UiConfig JSON. Returns 0 (AJNA_OK) when valid. */
    external fun uiConfigValidate(json: ByteArray): Int

    /** Create a scan session; returns an opaque handle (0 on failure). */
    external fun sessionCreate(minQualityFrames: Int, timeoutMs: Long): Long

    /** Feed one NV12 Y-plane frame; returns a pipeline status code. */
    external fun sessionProcessFrame(
        handle: Long,
        yPlane: ByteArray,
        width: Int,
        height: Int,
        yStride: Int,
        timestampUs: Long,
    ): Int

    /**
     * Push the OCR-parsed + host fields, signing the result with ML-DSA-65.
     * Fields are passed as parallel key/value arrays for a simple JNI ABI.
     */
    external fun sessionPushSigned(
        handle: Long,
        keys: Array<String>,
        values: Array<String>,
        documentType: String,
        issuingCountry: String,
        confidence: Float,
    ): Int

    /** Retrieve the completed, signed ScanResult as JSON. */
    external fun sessionResultJson(handle: Long): String?

    /** Destroy the session and free the Rust allocation. */
    external fun sessionDestroy(handle: Long): Int
}
