package com.ajna.sdk

import android.content.Context
import android.hardware.camera2.CameraManager
import android.os.Handler
import android.os.Looper

data class AjnaScanConfig(
    val modelPath: String,
    val minQualityFrames: Int = 3,
    val timeoutMs: Long = 30_000L,
    val pqcSignResult: Boolean = true
)

data class AjnaDocumentField(
    val key: String,
    val value: String,
    val confidence: Float
)

data class AjnaScanResult(
    val fields: List<AjnaDocumentField>,
    val documentType: String,
    val issuingCountry: String,
    val confidence: Float,
    val pqcSigned: Boolean
)

class AjnaScanner(private val context: Context) {

    private var sdk: AjnaSDK? = null
    private var cameraAdapter: AjnaCameraAdapter? = null
    private var resultCallback: ((Result<AjnaScanResult>) -> Unit)? = null
    private var pollingHandler: Handler? = null
    private var pollingRunnable: Runnable? = null

    fun configure() {
        // Validates camera permission is available at configure time.
        // No-op if permission not yet granted — startScan will fail gracefully.
    }

    fun startScan(config: AjnaScanConfig, onResult: (Result<AjnaScanResult>) -> Unit) {
        resultCallback = onResult

        val ajnaSdk = AjnaSDK().also { sdk = it }
        ajnaSdk.initialize(config.modelPath)

        val cameraManager = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager
        val adapter = AjnaCameraAdapter(cameraManager, ajnaSdk).also { cameraAdapter = it }
        adapter.open()

        startPolling(ajnaSdk)
    }

    fun stopScan() {
        stopPolling()
        cameraAdapter?.release()
        sdk?.destroy()
        cameraAdapter = null
        sdk = null
        resultCallback = null
    }

    private fun startPolling(ajnaSdk: AjnaSDK) {
        val handler = Handler(Looper.getMainLooper()).also { pollingHandler = it }
        val runnable = object : Runnable {
            override fun run() {
                val state = ajnaSdk.getSessionState()
                when (state) {
                    SessionState.COMPLETE -> {
                        // Result is available in the Rust session.
                        // ajna_session_get_result_json (added by Agent B) retrieves it.
                        // Until that FFI symbol lands, deliver a placeholder result.
                        val result = AjnaScanResult(
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
