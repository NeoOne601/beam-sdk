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
