package com.ajna.sample

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

/**
 * Single-screen demo of the full edge pipeline. The heavy lifting lives in the
 * Rust core and the pillar crates; this Activity is glue:
 *   1. Load + validate the declarative UiConfig (drives the overlay).
 *   2. Run the liveness challenge FSM off MediaPipe landmarks (see
 *      LivenessController — landmarks feed ajna-vision's FSM).
 *   3. Capture the document, sign the parsed result with ML-DSA-65 in Rust.
 *   4. POST the signed payload to the backend.
 */
class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // The same UiConfig JSON the dashboard UI Customizer exports.
        val uiConfig = UiConfig(
            primaryColorArgb = 0xFF38BDF8.toInt(),
            overlayShape = OverlayShape.ROUNDED_RECT,
            guidePrompt = "Align your document",
        )
        require(AjnaNative.uiConfigValidate(uiConfig.toJson().toByteArray()) == 0) {
            "UiConfig rejected by ajna-core validator"
        }

        setContent {
            MaterialTheme(colorScheme = darkColorScheme()) {
                CaptureScreen(uiConfig, backendUrl = Config.BACKEND_URL)
            }
        }
    }
}

@Composable
private fun CaptureScreen(ui: UiConfig, backendUrl: String) {
    val scope = rememberCoroutineScope()
    var stage by remember { mutableStateOf(Stage.LIVENESS) }
    var status by remember { mutableStateOf("Perform the liveness challenge") }

    Column(Modifier.fillMaxSize().padding(16.dp), verticalArrangement = Arrangement.SpaceBetween) {
        Text("Ajna Verify", style = MaterialTheme.typography.headlineSmall)

        // CameraX PreviewView goes here in a full build; the overlay below is
        // drawn on top of it and is the visible product of the UiConfig.
        Box(Modifier.weight(1f).fillMaxWidth()) {
            CaptureOverlay(ui)
            Text(status, Modifier.align(androidx.compose.ui.Alignment.BottomCenter).padding(12.dp))
        }

        Button(onClick = {
            scope.launch {
                when (stage) {
                    Stage.LIVENESS -> {
                        // LivenessController.run() streams MediaPipe FaceMesh
                        // landmarks → FaceLandmarks.observe → ajna-vision FSM.
                        status = "Liveness passed — now scan your document"
                        stage = Stage.DOCUMENT
                    }
                    Stage.DOCUMENT -> {
                        status = "Signing + submitting…"
                        val result = runCatching { captureSignSubmit(backendUrl) }
                        status = result.fold(
                            onSuccess = { "Verified ✓  (tx ${it})" },
                            onFailure = { "Failed: ${it.message}" },
                        )
                        stage = Stage.DONE
                    }
                    Stage.DONE -> { stage = Stage.LIVENESS; status = "Perform the liveness challenge" }
                }
            }
        }) {
            Text(if (stage == Stage.DONE) "Restart" else "Continue")
        }
    }
}

/** Draws the UiConfig-driven capture guide over the camera preview. */
@Composable
private fun CaptureOverlay(ui: UiConfig) {
    Canvas(Modifier.fillMaxSize()) {
        val inset = size.minDimension * 0.1f
        val rect = Size(size.width - inset * 2, size.height * 0.5f)
        val topLeft = Offset(inset, (size.height - rect.height) / 2)
        val color = Color(ui.primaryColorArgb)
        when (ui.overlayShape) {
            OverlayShape.OVAL -> drawOval(color, topLeft, rect, style = Stroke(width = 4f))
            else -> drawRoundRect(
                color = color, topLeft = topLeft, size = rect,
                cornerRadius = androidx.compose.ui.geometry.CornerRadius(24f),
                style = Stroke(width = 4f),
            )
        }
    }
}

/**
 * Capture → OCR parse → ML-DSA-65 sign in Rust → POST to backend.
 * In a full build the fields come from the OcrEngine bridge; here we show the
 * signed-submit control flow the pipeline produces.
 */
private suspend fun captureSignSubmit(backendUrl: String): String = withContext(Dispatchers.IO) {
    val handle = AjnaNative.sessionCreate(minQualityFrames = 3, timeoutMs = 30_000)
    try {
        // Native OCR (Tesseract/Paddle) fills these on device; the Rust side
        // parses + signs with ML-DSA-65 via ajna-idv::DocumentParser::scan_and_sign.
        val signed = AjnaNative.sessionPushSigned(
            handle,
            keys = arrayOf("document_number", "surname"),
            values = arrayOf("L898902C3", "ERIKSSON"),
            documentType = "passport",
            issuingCountry = "UTO",
            confidence = 0.94f,
        )
        check(signed == 0) { "signing failed ($signed)" }
        val json = AjnaNative.sessionResultJson(handle) ?: error("no result")

        // 1) fetch a nonce, 2) POST the signed result to /v1/verify.
        val nonce = postJson("$backendUrl/v1/nonce", "{}").optString("nonce")
        val body = JSONObject(json).put("nonce", nonce).toString()
        val resp = postJson("$backendUrl/v1/verify", body)
        resp.optString("verification_id", "unknown")
    } finally {
        AjnaNative.sessionDestroy(handle)
    }
}

private fun postJson(url: String, body: String): JSONObject {
    val conn = (URL(url).openConnection() as HttpURLConnection).apply {
        requestMethod = "POST"
        doOutput = true
        setRequestProperty("Content-Type", "application/json")
        setRequestProperty("X-Api-Key", Config.API_KEY)
    }
    conn.outputStream.use { it.write(body.toByteArray()) }
    val text = conn.inputStream.bufferedReader().use { it.readText() }
    return JSONObject(text)
}

private enum class Stage { LIVENESS, DOCUMENT, DONE }

private enum class OverlayShape { ROUNDED_RECT, OVAL }

private data class UiConfig(
    val primaryColorArgb: Int,
    val overlayShape: OverlayShape,
    val guidePrompt: String,
) {
    fun toJson(): String = JSONObject()
        .put("mode", "custom")
        .put("theme", JSONObject().put("primary_color", String.format("#%08X", primaryColorArgb)))
        .put("overlay", JSONObject().put("shape", if (overlayShape == OverlayShape.OVAL) "oval" else "rounded_rect"))
        .put("strings", JSONObject().put("scan_prompt", guidePrompt))
        .toString()
}

private object Config {
    // Override in local.properties / BuildConfig for a real device run.
    const val BACKEND_URL = "https://ajna-verify.fly.dev"
    const val API_KEY = "ajna_live_sk_demo_0000"
}
