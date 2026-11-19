plugins {
    id("com.android.application")
}

android {
    namespace = "io.github.aminuddinihsan20.bitchord"
    compileSdk = 36

    defaultConfig {
        applicationId = "io.github.aminuddinihsan20.bitchord"
        // AAudio, which cpal plays through, needs API 26.
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "2.0.1"
        // The Rust library is built for arm64 only (phones, and the emulator on Apple silicon).
        ndk {
            abiFilters += "arm64-v8a"
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    // The version android-activity 0.6 (in eframe/winit) is written against; don't add
    // its native "prefab" glue, android-activity has its own.
    implementation("androidx.games:games-activity:4.4.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    // Lets the sign-in WebView leave out the header that names the app.
    implementation("androidx.webkit:webkit:1.14.0")
}
