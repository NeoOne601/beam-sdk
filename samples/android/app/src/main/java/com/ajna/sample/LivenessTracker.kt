package com.ajna.sample

import android.content.Context
import androidx.camera.core.ImageProxy
import com.google.mediapipe.framework.image.BitmapImageBuilder
import com.google.mediapipe.tasks.core.BaseOptions
import com.google.mediapipe.tasks.vision.core.RunningMode
import com.google.mediapipe.tasks.vision.facelandmarker.FaceLandmarker
import com.google.mediapipe.tasks.vision.facelandmarker.FaceLandmarker.FaceLandmarkerOptions

/**
 * Thin wrapper over MediaPipe FaceLandmarker for the liveness challenge.
 * The model ships at `assets/face_landmarker.task` (no runtime download).
 * Blink is read off the built-in face blendshapes (eyeBlinkLeft/Right).
 */
object LivenessTracker {

    private const val BLINK_THRESHOLD = 0.5f

    fun build(context: Context): FaceLandmarker {
        val base = BaseOptions.builder()
            .setModelAssetPath("face_landmarker.task")
            .build()
        val options = FaceLandmarkerOptions.builder()
            .setBaseOptions(base)
            .setRunningMode(RunningMode.IMAGE)
            .setOutputFaceBlendshapes(true)   // needed for eyeBlink* categories
            .setNumFaces(1)
            .build()
        return FaceLandmarker.createFromOptions(context, options)
    }

    /**
     * Returns true when a blink is detected in this frame (either eye's
     * blink blendshape above threshold). Runs synchronously in IMAGE mode.
     */
    fun detectBlink(landmarker: FaceLandmarker, image: ImageProxy): Boolean {
        val bitmap = image.toBitmap() ?: return false
        val mpImage = BitmapImageBuilder(bitmap).build()
        val result = landmarker.detect(mpImage)
        val shapes = result.faceBlendshapes().orElse(null)?.firstOrNull() ?: return false
        return shapes.any { cat ->
            (cat.categoryName() == "eyeBlinkLeft" || cat.categoryName() == "eyeBlinkRight") &&
                cat.score() >= BLINK_THRESHOLD
        }
    }
}
