plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
}

/** Per-machine settings from android/local.properties (spec 2026-10-21 §3.4); see local.properties.example. */
val settings = LocalSettings(providers, rootProject.file("local.properties"))
val serverUrl = settings.get("whichride.serverUrl") ?: ""
val cloudProjectNumber = settings.get("whichride.cloudProjectNumber") ?: "0"
val registrationKey = settings.get("whichride.registrationKey") ?: ""
val uploadStoreFile = settings.get("whichride.upload.storeFile")
val uploadStorePassword = settings.get("whichride.upload.storePassword")
val uploadKeyAlias = settings.get("whichride.upload.keyAlias")
val uploadKeyPassword = settings.get("whichride.upload.keyPassword")
val requireUploadKey = settings.get("whichride.requireUploadKey").toBoolean()

require(serverUrl.isEmpty() || serverUrl.matches(Regex("""https://[^\s"]+|http://(localhost|10\.0\.2\.2)(:\d+)?"""))) {
    "whichride.serverUrl must be https:// (or http://localhost / http://10.0.2.2 for a local server)"
}
require(cloudProjectNumber.matches(Regex("""\d{1,20}"""))) { "whichride.cloudProjectNumber must be digits" }
require(registrationKey.isEmpty() || registrationKey.matches(Regex("""[A-Za-z0-9+/=_-]{32,}"""))) {
    "whichride.registrationKey must be at least 32 characters of A-Z a-z 0-9 + / = _ -"
}
require(!(requireUploadKey && registrationKey.isNotEmpty())) {
    "A Play release must not contain whichride.registrationKey (spec 2026-10-21 §3.2)"
}
require(uploadStoreFile != null || !requireUploadKey) {
    "No upload key: set whichride.upload.storeFile, storePassword, keyAlias and keyPassword"
}

logger.lifecycle(
    "whichride: " + listOf(
        if (serverUrl.isEmpty()) "no server (built-in prices only)" else "server $serverUrl",
        "attestation: " + when {
            registrationKey.isNotEmpty() -> "registration key"
            cloudProjectNumber == "0" -> "none"
            else -> "Play Integrity"
        },
        "signing: " + if (uploadStoreFile != null) "upload key" else "debug key",
    ).joinToString(" · ") + if (serverUrl.isEmpty()) " · see android/local.properties.example" else "",
)

android {
    namespace = "io.github.locadani.whichride"
    compileSdk = 37
    defaultConfig {
        applicationId = "io.github.locadani.whichride" // decision 41; never changes
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        buildConfigField("String", "SERVER_URL", "\"$serverUrl\"")
        buildConfigField("long", "CLOUD_PROJECT_NUMBER", "${cloudProjectNumber}L")
        buildConfigField("String", "REGISTRATION_KEY", "\"$registrationKey\"")
        // JNA also ships libraries for ABIs no phone uses any more (spike finding).
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64") }
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
    signingConfigs {
        if (uploadStoreFile != null) {
            create("upload") {
                storeFile = file(uploadStoreFile)
                storePassword = requireNotNull(uploadStorePassword) { "whichride.upload.storePassword is missing" }
                keyAlias = requireNotNull(uploadKeyAlias) { "whichride.upload.keyAlias is missing" }
                keyPassword = requireNotNull(uploadKeyPassword) { "whichride.upload.keyPassword is missing" }
            }
        }
    }
    buildTypes {
        release {
            signingConfig = signingConfigs.findByName("upload") ?: signingConfigs.getByName("debug")
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    androidResources {
        // Android 13+ lists the app's languages in its per-app language settings (R16, R17).
        generateLocaleConfig = true
    }
    testOptions {
        unitTests.isIncludeAndroidResources = true
    }
}

val bundledFiles = tasks.register<CopyIntoDirectory>("bundledFiles") {
    sources.from(
        repoRoot.file("crates/server/catalogue.json"),
        repoRoot.file("design/fonts/Sora-OFL.txt"),
        repoRoot.file("design/fonts/JetBrainsMono-OFL.txt"),
        repoRoot.file("LICENSE"),
    )
    targets.set(
        mapOf(
            "catalogue.json" to "seed-catalogue.json",
            "Sora-OFL.txt" to "licences/Sora-OFL.txt",
            "JetBrainsMono-OFL.txt" to "licences/JetBrainsMono-OFL.txt",
            "LICENSE" to "licences/whichride-LICENSE.txt",
        ),
    )
    outputDir.set(layout.buildDirectory.dir("generated/bundled"))
}

androidComponents.onVariants { variant ->
    variant.sources.assets?.addGeneratedSourceDirectory(bundledFiles, CopyIntoDirectory::outputDir)
}

// Every build fails while local.properties.example and the settings the build uses disagree.
val checkLocalSettings = tasks.register<CheckLocalSettings>("checkLocalSettings") {
    names.set(provider { settings.names })
    template.set(rootProject.layout.projectDirectory.file("local.properties.example"))
    buildScripts.from(
        rootProject.fileTree(rootProject.projectDir) {
            include("*.gradle.kts", "*/*.gradle.kts", "buildSrc/src/main/**/*.kt")
            exclude("**/build/**")
        },
    )
}
tasks.named("preBuild") { dependsOn(checkLocalSettings) }

useHostRustInTests()

dependencies {
    implementation(project(":designsystem"))
    implementation(project(":core"))
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.material3)
    implementation(libs.compose.material.icons.core)
    implementation(libs.compose.ui.tooling.preview)
    debugImplementation(libs.compose.ui.tooling)
    implementation(libs.activity.compose)
    implementation(libs.lifecycle.viewmodel.compose)
    implementation(libs.lifecycle.runtime.compose)
    implementation(libs.core.ktx)
    implementation(libs.okhttp)
    implementation(libs.okhttp.coroutines)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.kotlinx.coroutines.android)
    implementation(libs.kotlinx.coroutines.play.services)
    implementation(libs.play.integrity)

    testImplementation(libs.junit)
    testImplementation(libs.jna)
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.okhttp.mockwebserver)
    testImplementation(libs.robolectric)
    testImplementation(libs.androidx.test.core)
    testImplementation(libs.androidx.test.ext.junit)
    testImplementation(platform(libs.compose.bom))
    testImplementation(libs.compose.ui.test.junit4)
    debugImplementation(libs.compose.ui.test.manifest)
}
