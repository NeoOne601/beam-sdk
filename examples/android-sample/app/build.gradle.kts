// Ajna Android sample — module build script.
// Requires the Android Gradle Plugin + a device (camera). Not built in CI.
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
        // ajna-core .so ships per-ABI under app/src/main/jniLibs/<abi>/.
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a") }
    }

    buildFeatures { compose = true }
    composeOptions { kotlinCompilerExtensionVersion = "1.5.14" }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
}

dependencies {
    implementation(platform("androidx.compose:compose-bom:2024.09.00"))
    implementation("androidx.activity:activity-compose:1.9.2")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")

    // CameraX for front (liveness) + back (document) capture.
    val camerax = "1.3.4"
    implementation("androidx.camera:camera-camera2:$camerax")
    implementation("androidx.camera:camera-lifecycle:$camerax")
    implementation("androidx.camera:camera-view:$camerax")

    // MediaPipe Face Landmarker → drives the ajna-vision liveness FSM.
    implementation("com.google.mediapipe:tasks-vision:0.10.14")
}
