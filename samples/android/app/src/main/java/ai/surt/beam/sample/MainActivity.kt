package ai.surt.beam.sample

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Bundle
import android.widget.Button
import android.widget.TextView
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat

/**
 * Main entry point for the Beam Verify sample app.
 * Displays SDK version info and provides navigation to the scan flow.
 */
class MainActivity : AppCompatActivity() {

    private val cameraPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission()
    ) { granted ->
        if (granted) {
            navigateToScan()
        } else {
            Toast.makeText(this, "Camera permission is required for document scanning", Toast.LENGTH_LONG).show()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val tvVersion = findViewById<TextView>(R.id.tvVersion)
        tvVersion.text = "Beam Verify SDK v0.1.0"

        val tvDescription = findViewById<TextView>(R.id.tvDescription)
        tvDescription.text = buildString {
            appendLine("Post-quantum cryptographically signed")
            appendLine("identity verification SDK")
            appendLine()
            appendLine("• ML-DSA (FIPS 204) result signing")
            appendLine("• On-device quality gates")
            appendLine("• TFLite GPU inference")
            appendLine("• mlock()-protected key storage")
        }

        val btnScan = findViewById<Button>(R.id.btnStartScan)
        btnScan.setOnClickListener {
            checkCameraPermissionAndScan()
        }

        val btnSettings = findViewById<Button>(R.id.btnSettings)
        btnSettings.setOnClickListener {
            startActivity(Intent(this, SettingsActivity::class.java))
        }
    }

    private fun checkCameraPermissionAndScan() {
        when {
            ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA)
                == PackageManager.PERMISSION_GRANTED -> {
                navigateToScan()
            }
            else -> {
                cameraPermissionLauncher.launch(Manifest.permission.CAMERA)
            }
        }
    }

    private fun navigateToScan() {
        startActivity(Intent(this, ScanActivity::class.java))
    }
}
