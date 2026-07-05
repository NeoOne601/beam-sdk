package com.ajna.sample

import android.os.Bundle
import android.widget.Button
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import kotlinx.coroutines.*
import java.net.HttpURLConnection
import java.net.URL
import java.util.UUID

/**
 * Displays scan results and provides backend verification.
 *
 * Shows:
 * - Extracted document fields with confidence scores
 * - PQC signature status
 * - Backend verification button (nonce → verify flow)
 */
class ResultActivity : AppCompatActivity() {

    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_result)

        val container = findViewById<LinearLayout>(R.id.fieldContainer)
        val tvDocType = findViewById<TextView>(R.id.tvDocumentType)
        val tvConfidence = findViewById<TextView>(R.id.tvConfidence)
        val tvPqcStatus = findViewById<TextView>(R.id.tvPqcStatus)
        val btnVerify = findViewById<Button>(R.id.btnVerifyBackend)
        val btnNewScan = findViewById<Button>(R.id.btnNewScan)

        // Extract data from intent
        val docType = intent.getStringExtra("document_type") ?: "unknown"
        val country = intent.getStringExtra("issuing_country") ?: "UNK"
        val confidence = intent.getFloatExtra("confidence", 0f)
        val pqcSigned = intent.getBooleanExtra("pqc_signed", false)

        tvDocType.text = "${docType.uppercase()} — $country"
        tvConfidence.text = "Confidence: ${String.format("%.1f%%", confidence * 100)}"
        tvPqcStatus.text = if (pqcSigned) {
            "✓ PQC Signed (ML-DSA Level 3)"
        } else {
            "✗ Not signed"
        }

        // Display document fields
        val fields = listOf(
            "surname" to intent.getStringExtra("surname"),
            "given_names" to intent.getStringExtra("given_names"),
            "date_of_birth" to intent.getStringExtra("date_of_birth"),
            "document_number" to intent.getStringExtra("document_number"),
            "expiry_date" to intent.getStringExtra("expiry_date"),
        )

        for ((key, value) in fields) {
            if (value != null) {
                val fieldView = TextView(this).apply {
                    text = "${key.replace('_', ' ').uppercase()}: $value"
                    textSize = 16f
                    setPadding(0, 8, 0, 8)
                }
                container.addView(fieldView)
            }
        }

        btnVerify.setOnClickListener {
            btnVerify.isEnabled = false
            btnVerify.text = "Verifying..."
            verifyWithBackend(btnVerify)
        }

        btnNewScan.setOnClickListener {
            finish()
        }
    }

    /**
     * Performs nonce → verify flow against the Ajna Verify backend.
     * 1. POST /v1/nonce to get a single-use nonce
     * 2. POST /v1/verify with the nonce and scan result
     */
    private fun verifyWithBackend(btnVerify: Button) {
        scope.launch {
            try {
                val baseUrl = getSharedPreferences("ajna_settings", MODE_PRIVATE)
                    .getString("backend_url", "http://10.0.2.2:8080") ?: "http://10.0.2.2:8080"

                // Step 1: Get nonce
                val sessionId = UUID.randomUUID().toString()
                val nonceResponse = withContext(Dispatchers.IO) {
                    postJson("$baseUrl/v1/nonce", """{"session_id":"$sessionId"}""")
                }

                // Step 2: Verify (simplified — real impl sends full ScanResult with signature)
                val verifyResponse = withContext(Dispatchers.IO) {
                    postJson("$baseUrl/v1/verify", """
                        {
                            "session_id": "$sessionId",
                            "nonce": "$nonceResponse",
                            "scan_result": {
                                "fields": [],
                                "document_type": "passport",
                                "issuing_country": "USA",
                                "confidence": 0.97,
                                "pqc_signature": "",
                                "pqc_public_key": ""
                            }
                        }
                    """.trimIndent())
                }

                btnVerify.text = "✓ Verified"
                Toast.makeText(this@ResultActivity, "Backend verification complete", Toast.LENGTH_SHORT).show()

            } catch (e: Exception) {
                btnVerify.text = "Verify with Backend"
                btnVerify.isEnabled = true
                Toast.makeText(this@ResultActivity, "Verification failed: ${e.message}", Toast.LENGTH_LONG).show()
            }
        }
    }

    private fun postJson(url: String, json: String): String {
        val conn = URL(url).openConnection() as HttpURLConnection
        conn.requestMethod = "POST"
        conn.setRequestProperty("Content-Type", "application/json")
        conn.doOutput = true
        conn.connectTimeout = 10_000
        conn.readTimeout = 10_000
        conn.outputStream.use { it.write(json.toByteArray()) }
        return conn.inputStream.bufferedReader().readText()
    }

    override fun onDestroy() {
        super.onDestroy()
        scope.cancel()
    }
}
