package com.ajna.sample

import android.os.Bundle
import android.view.View
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import android.content.Intent

/**
 * Camera scanning activity.
 * Displays live camera preview with quality gate status overlay.
 *
 * Architecture note:
 *   Camera (Kotlin) → AjnaCameraAdapter → Quality Gates (Rust via JNI)
 *   → TFLite inference (C++ GPU delegate) → ajna_session_push_result (Rust FFI)
 *   → PQC signing (Rust) → ScanResult delivered to this activity
 */
class ScanActivity : AppCompatActivity() {

    // In production, these would be Ajna SDK handles obtained via JNI
    private var sessionHandle: Long = 0L
    private var gateHandle: Long = 0L

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_scan)

        val tvStatus = findViewById<TextView>(R.id.tvGateStatus)
        val tvFrameCount = findViewById<TextView>(R.id.tvFrameCount)
        val progressBar = findViewById<ProgressBar>(R.id.progressScan)

        tvStatus.text = "Initialising camera..."
        tvFrameCount.text = "Quality frames: 0 / 3"
        progressBar.visibility = View.VISIBLE

        // In production:
        // 1. Create AjnaCameraAdapter with camera2 API
        // 2. Create Rust session via ajna_session_create()
        // 3. Create quality gate via ajna_gate_create()
        // 4. Start session via ajna_session_start()
        // 5. Feed frames through gate → inference → push_result pipeline
        // 6. On SessionState.Complete, navigate to ResultActivity

        simulateScanFlow(tvStatus, tvFrameCount, progressBar)
    }

    /**
     * Simulates the scan flow for demonstration purposes.
     * In production, this is driven by real camera frames and Ajna SDK callbacks.
     */
    private fun simulateScanFlow(
        tvStatus: TextView,
        tvFrameCount: TextView,
        progressBar: ProgressBar
    ) {
        val handler = android.os.Handler(mainLooper)
        val gateNames = listOf("BlurCheck", "ExposureCheck", "MotionCheck", "BoundaryCheck", "Accepted")

        // Simulate gate evaluations
        gateNames.forEachIndexed { index, gate ->
            handler.postDelayed({
                tvStatus.text = "Gate: $gate"
                if (gate == "Accepted") {
                    tvFrameCount.text = "Quality frames: ${index.coerceAtMost(3)} / 3"
                }
            }, (index + 1) * 800L)
        }

        // Simulate inference
        handler.postDelayed({
            tvStatus.text = "Running ML inference (TFLite GPU)..."
            tvFrameCount.text = "Quality frames: 3 / 3"
        }, 5000L)

        // Simulate PQC signing
        handler.postDelayed({
            tvStatus.text = "Signing with ML-DSA Level 3..."
        }, 6000L)

        // Navigate to result
        handler.postDelayed({
            progressBar.visibility = View.GONE
            tvStatus.text = "Scan complete!"

            val intent = Intent(this, ResultActivity::class.java).apply {
                putExtra("document_type", "passport")
                putExtra("issuing_country", "USA")
                putExtra("confidence", 0.97f)
                putExtra("surname", "SMITH")
                putExtra("given_names", "JOHN MICHAEL")
                putExtra("date_of_birth", "1990-05-15")
                putExtra("document_number", "C01X00T47")
                putExtra("expiry_date", "2030-05-14")
                putExtra("pqc_signed", true)
            }
            startActivity(intent)
            finish()
        }, 7000L)
    }

    override fun onDestroy() {
        super.onDestroy()
        // In production: ajna_session_destroy(sessionHandle)
        // In production: ajna_gate_destroy(gateHandle)
    }
}
