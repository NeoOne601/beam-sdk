package ai.surt.beam.sample

import android.os.Bundle
import android.widget.Button
import android.widget.EditText
import android.widget.Switch
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

/**
 * Settings screen for configuring the Beam Verify sample app.
 */
class SettingsActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)

        val prefs = getSharedPreferences("beam_settings", MODE_PRIVATE)

        val etBackendUrl = findViewById<EditText>(R.id.etBackendUrl)
        val switchPqc = findViewById<Switch>(R.id.switchPqcSigning)
        val switchRawMrz = findViewById<Switch>(R.id.switchRawMrz)
        val btnSave = findViewById<Button>(R.id.btnSaveSettings)

        // Load saved settings
        etBackendUrl.setText(prefs.getString("backend_url", "http://10.0.2.2:8080"))
        switchPqc.isChecked = prefs.getBoolean("pqc_signing", true)
        switchRawMrz.isChecked = prefs.getBoolean("include_raw_mrz", false)

        btnSave.setOnClickListener {
            prefs.edit().apply {
                putString("backend_url", etBackendUrl.text.toString())
                putBoolean("pqc_signing", switchPqc.isChecked)
                putBoolean("include_raw_mrz", switchRawMrz.isChecked)
                apply()
            }
            Toast.makeText(this, "Settings saved", Toast.LENGTH_SHORT).show()
            finish()
        }
    }
}
