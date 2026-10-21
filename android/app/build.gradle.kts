plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
}

/** Not secrets, but deployment-specific: set them in ~/.gradle/gradle.properties (R9). */
val serverUrl = providers.gradleProperty("whichride.serverUrl").getOrElse("")
val cloudProjectNumber = providers.gradleProperty("whichride.cloudProjectNumber").getOrElse("0")
require(serverUrl.isEmpty() || serverUrl.matches(Regex("""https://[^\s"]+|http://(localhost|10\.0\.2\.2)(:\d+)?"""))) {
    "whichride.serverUrl must be https:// (or http://localhost / http://10.0.2.2 for a local server)"
}
require(cloudProjectNumber.matches(Regex("""\d{1,20}"""))) { "whichride.cloudProjectNumber must be digits" }

/**
 * The Play upload key: from ~/.gradle/gradle.properties locally, or WHICHRIDE_UPLOAD_* in the
 * release workflow. Without it a release build is signed with the debug key, so R8 builds can
 * still be installed on a phone; the release workflow sets whichride.requireUploadKey (plan 06).
 */
fun upload(name: String): String? =
    providers.gradleProperty("whichride.upload.$name")
        .orElse(providers.environmentVariable("WHICHRIDE_UPLOAD_" + name.uppercase()))
        .orNull?.takeIf { it.isNotBlank() }
val uploadStoreFile = upload("storeFile")
val requireUploadKey = providers.gradleProperty("whichride.requireUploadKey").map(String::toBoolean).getOrElse(false)
require(uploadStoreFile != null || !requireUploadKey) {
    "No upload key: set whichride.upload.storeFile, storePassword, keyAlias and keyPassword"
}

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
                storePassword = requireNotNull(upload("storePassword")) { "whichride.upload.storePassword is missing" }
                keyAlias = requireNotNull(upload("keyAlias")) { "whichride.upload.keyAlias is missing" }
                keyPassword = requireNotNull(upload("keyPassword")) { "whichride.upload.keyPassword is missing" }
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
