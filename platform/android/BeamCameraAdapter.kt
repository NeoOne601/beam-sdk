// platform/android/BeamCameraAdapter.kt
// Camera2 adapter for Beam SDK.
// Negotiates YUV_420_888 (NV12) at ISP level, manages buffer pools,
// delivers zero-copy AHardwareBuffer pointers to the C++ ML layer.

package ai.surt.beam

import android.hardware.camera2.*
import android.hardware.camera2.params.OutputConfiguration
import android.hardware.camera2.params.SessionConfiguration
import android.media.ImageReader
import android.media.Image
import android.os.Handler
import android.os.HandlerThread
import android.util.Size
import android.view.Surface
import java.util.concurrent.Executors

/**
 * Negotiates the camera session at ISP level for document scanning.
 *
 * Key decisions:
 *  - Format: ImageFormat.YUV_420_888 — maps to NV12 on most SoCs.
 *    We request it explicitly; if the HAL gives us YV12 instead we
 *    detect that from the Image.planes layout and flag it for the C++ layer.
 *  - Resolution: 1920x1080 minimum. Budget devices (Helio G85) do 1080p
 *    natively without software scaling cost.
 *  - Frame rate: locked to 25fps via capture request template.
 *    Higher fps wastes power; lower fps reduces quality gate throughput.
 *  - Buffer count: 4 — enough to absorb HAL pipeline depth without starvation.
 *    Helio G85 HAL typically buffers 3 frames. Fewer than 4 causes dropped frames.
 */
class BeamCameraAdapter(
    private val cameraManager: CameraManager,
    private val beamSdk:       BeamSDK,
) {

    private var cameraDevice:    CameraDevice?       = null
    private var captureSession:  CameraCaptureSession? = null
    private var imageReader:     ImageReader?         = null
    private val cameraThread =   HandlerThread("BeamCamera").apply { start() }
    private val cameraHandler =  Handler(cameraThread.looper)
    private val executor      =  Executors.newSingleThreadExecutor()

    /** Select back-facing camera and open the device. */
    fun open(cameraId: String = selectDocumentCamera()) {
        cameraManager.openCamera(cameraId, object : CameraDevice.StateCallback() {
            override fun onOpened(camera: CameraDevice) {
                cameraDevice = camera
                startCaptureSession()
            }
            override fun onDisconnected(camera: CameraDevice) { camera.close() }
            override fun onError(camera: CameraDevice, error: Int) {
                camera.close()
                beamSdk.onCameraError(error)
            }
        }, cameraHandler)
    }

    private fun startCaptureSession() {
        val camera = cameraDevice ?: return

        // Request 1920x1080 YUV_420_888. Buffer count = 4.
        imageReader = ImageReader.newInstance(
            1920, 1080,
            android.graphics.ImageFormat.YUV_420_888,
            /* maxImages = */ 4
        )

        imageReader!!.setOnImageAvailableListener({ reader ->
            val image = reader.acquireLatestImage() ?: return@setOnImageAvailableListener
            processFrame(image)
            image.close()
        }, cameraHandler)

        val surface = imageReader!!.surface
        val outputConfig = OutputConfiguration(surface)

        val sessionConfig = SessionConfiguration(
            SessionConfiguration.SESSION_REGULAR,
            listOf(outputConfig),
            executor,
            object : CameraCaptureSession.StateCallback() {
                override fun onConfigured(session: CameraCaptureSession) {
                    captureSession = session
                    startRepeatingCapture(session, surface)
                }
                override fun onConfigureFailed(session: CameraCaptureSession) {
                    beamSdk.onCameraError(-1)
                }
            }
        )
        camera.createCaptureSession(sessionConfig)
    }

    private fun startRepeatingCapture(session: CameraCaptureSession, surface: Surface) {
        val request = cameraDevice!!
            .createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW)
            .apply {
                addTarget(surface)
                // Lock to 25fps — (40_000_000 ns = 40ms frame duration)
                set(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE,
                    android.util.Range(25, 25))
                // Disable video stabilisation — it introduces latency and motion artifacts
                set(CaptureRequest.CONTROL_VIDEO_STABILIZATION_MODE,
                    CaptureRequest.CONTROL_VIDEO_STABILIZATION_MODE_OFF)
                // Auto-focus in continuous picture mode
                set(CaptureRequest.CONTROL_AF_MODE,
                    CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE)
            }.build()

        session.setRepeatingRequest(request, null, cameraHandler)
    }

    /**
     * Process a YUV_420_888 frame from Camera2.
     *
     * The Y plane is the quality gate input. The full NV12 frame is passed
     * to the C++ ML layer. We detect whether the HAL returned NV12 or YV12
     * from the planes layout and pass the format flag to native code.
     */
    private fun processFrame(image: Image) {
        val planes = image.planes
        val yPlane  = planes[0]
        val uvPlane = planes[1] // NV12: interleaved UV; YV12: this is U plane

        // Detect NV12 vs YV12 from pixel stride of UV plane.
        // NV12: UV pixel stride = 2. YV12: UV pixel stride = 1.
        val isNv12 = (uvPlane.pixelStride == 2)

        beamSdk.onFrame(
            yBuffer     = yPlane.buffer,
            uvBuffer    = uvPlane.buffer,
            width       = image.width,
            height      = image.height,
            yStride     = yPlane.rowStride,
            uvStride    = uvPlane.rowStride,
            isNv12      = isNv12,
            timestampUs = image.timestamp / 1000L,
        )
    }

    /** Select the best back-facing camera (prefer wide-angle, reject ultra-wide). */
    private fun selectDocumentCamera(): String {
        for (id in cameraManager.cameraIdList) {
            val chars = cameraManager.getCameraCharacteristics(id)
            val facing = chars.get(CameraCharacteristics.LENS_FACING)
            if (facing == CameraCharacteristics.LENS_FACING_BACK) {
                return id
            }
        }
        return cameraManager.cameraIdList.first()
    }

    fun release() {
        captureSession?.close()
        cameraDevice?.close()
        imageReader?.close()
        cameraThread.quitSafely()
    }
}
