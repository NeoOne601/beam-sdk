// Ajna Android sample — app module.
// Links the Rust core via app/libs/AjnaSDK-0.1.0-release.aar; OCR via ML Kit;
// liveness via MediaPipe FaceLandmarker (assets/face_landmarker.task).
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.ajna.sample"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.ajna.sample"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64") }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
    // MediaPipe .task assets must not be compressed.
    androidResources { noCompress += "task" }
    buildTypes {
        release { isMinifyEnabled = false }
    }
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.constraintlayout:constraintlayout:2.1.4")

    // Ajna SDK (Rust core JNI) — local AAR under app/libs/
    implementation(files("libs/AjnaSDK-0.1.0-release.aar"))

    // CameraX
    val camerax = "1.3.4"
    implementation("androidx.camera:camera-camera2:$camerax")
    implementation("androidx.camera:camera-lifecycle:$camerax")
    implementation("androidx.camera:camera-view:$camerax")

    // ML Kit Text Recognition (document OCR)
    implementation("com.google.mlkit:text-recognition:16.0.1")

    // MediaPipe Face Landmarker (liveness)
    implementation("com.google.mediapipe:tasks-vision:0.10.14")
}
