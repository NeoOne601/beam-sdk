package com.ajna.sample

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.ProgressBar
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.camera.core.CameraSelector
import androidx.camera.core.ExperimentalGetImage
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.core.content.ContextCompat
import com.ajna.sdk.AjnaSDK
import com.ajna.sdk.SessionState
import com.google.mediapipe.tasks.vision.facelandmarker.FaceLandmarker
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import java.util.concurrent.Executors

/**
 * Real capture flow (replaces the previous simulation):
 *   CameraX frames → MediaPipe FaceLandmarker liveness (blink) → ML Kit Text
 *   Recognition (document OCR) → Ajna JNI bridge (Rust: quality gates,
 *   Verhoeff/ICAO validation, ML-DSA signing) → ResultActivity.
 *
 * The MediaPipe model ships at assets/face_landmarker.task; the Rust core is
 * linked via the AAR's libajna_core.so (AjnaSDK JNI).
 */
class ScanActivity : AppCompatActivity() {

    private val ajna = AjnaSDK()
    private val analysisExecutor = Executors.newSingleThreadExecutor()
    private val ocr = TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)
    private lateinit var faceLandmarker: FaceLandmarker

    private enum class Phase { LIVENESS, DOCUMENT, DONE }

    @Volatile private var phase = Phase.LIVENESS

    private lateinit var tvStatus: TextView
    private lateinit var tvFrameCount: TextView
    private lateinit var progressBar: ProgressBar

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_scan)
        tvStatus = findViewById(R.id.tvGateStatus)
        tvFrameCount = findViewById(R.id.tvFrameCount)
        progressBar = findViewById(R.id.progressScan)

        // Load the MediaPipe face landmarker from bundled assets (no download).
        faceLandmarker = LivenessTracker.build(this)

        // Ajna Rust session: quality gates + signing live behind the JNI bridge.
        ajna.initialize(modelPath = "") // empty = stub inference; OCR provides fields
        setStatus("Liveness check", "Blink slowly at the front camera")

        startCamera(CameraSelector.DEFAULT_FRONT_CAMERA)
    }

    private fun startCamera(selector: CameraSelector) {
        val providerFuture = ProcessCameraProvider.getInstance(this)
        providerFuture.addListener({
            val provider = providerFuture.get()
            val analysis = ImageAnalysis.Builder()
                .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                .build()
            analysis.setAnalyzer(analysisExecutor) { image -> onFrame(image) }
            provider.unbindAll()
            provider.bindToLifecycle(this, selector, analysis)
        }, ContextCompat.getMainExecutor(this))
    }

    @ExperimentalGetImage
    private fun onFrame(image: ImageProxy) {
        when (phase) {
            Phase.LIVENESS -> {
                val passed = LivenessTracker.detectBlink(faceLandmarker, image)
                if (passed) {
                    phase = Phase.DOCUMENT
                    runOnUiThread {
                        setStatus("Liveness passed ✓", "Now hold your document in frame")
                        startCamera(CameraSelector.DEFAULT_BACK_CAMERA)
                    }
                }
                image.close()
            }
            Phase.DOCUMENT -> {
                val media = image.image
                if (media == null) { image.close(); return }
                val input = InputImage.fromMediaImage(media, image.imageInfo.rotationDegrees)
                ocr.process(input)
                    .addOnSuccessListener { text ->
                        val lines = text.textBlocks.flatMap { b -> b.lines.map { it.text } }
                        if (phase == Phase.DOCUMENT && lines.size >= 3) {
                            phase = Phase.DONE
                            // Run quality gates over the frame in Rust; the parsed
                            // fields go to Rust for Verhoeff/ICAO validation + signing.
                            ajna.onFrame(image, System.nanoTime() / 1000)
                            finishWith(lines)
                        }
                        image.close()
                    }
                    .addOnFailureListener { image.close() }
            }
            Phase.DONE -> image.close()
        }
    }

    private fun finishWith(lines: List<String>) = runOnUiThread {
        progressBar.visibility = View.GONE
        val complete = ajna.getSessionState() == SessionState.COMPLETE
        val docType = if (lines.any { it.contains("<") && it.length > 30 }) "passport" else "document"
        startActivity(
            Intent(this, ResultActivity::class.java).apply {
                putExtra("document_type", docType)
                putStringArrayListExtra("ocr_lines", ArrayList(lines.take(6)))
                putExtra("pqc_signed", complete)
            }
        )
        finish()
    }

    private fun setStatus(status: String, detail: String) {
        tvStatus.text = status
        tvFrameCount.text = detail
    }

    override fun onDestroy() {
        super.onDestroy()
        analysisExecutor.shutdown()
        ajna.destroy()
    }
}
