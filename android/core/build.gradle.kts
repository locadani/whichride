plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "io.github.locadani.whichride.core"
    compileSdk = 37
    ndkVersion = libs.versions.ndk.get()
    defaultConfig {
        minSdk = 26
        consumerProguardFiles("consumer-rules.pro")
    }
}

val androidAbis = listOf("arm64-v8a", "armeabi-v7a", "x86_64")

val cargoNdkBuild = tasks.register<CargoNdkBuild>("cargoNdkBuild") {
    sources.from(rustSources())
    workspace.set(repoRoot)
    abis.set(androidAbis)
    minSdk.set(26)
    ndkDirectory.set(androidComponents.sdkComponents.ndkDirectory)
    outputDir.set(layout.buildDirectory.dir("rust/jniLibs"))
}

val cargoHostBuild = tasks.register<CargoHostBuild>("cargoHostBuild") {
    sources.from(rustSources())
    workspace.set(repoRoot)
    library.set(repoRoot.file("target/debug/$hostLibraryName"))
}

val uniffiBindgen = tasks.register<UniffiBindgen>("uniffiBindgen") {
    sources.from(rustSources())
    workspace.set(repoRoot)
    library.set(cargoHostBuild.flatMap { it.library })
    outputDir.set(layout.buildDirectory.dir("generated/uniffi"))
}

androidComponents.onVariants { variant ->
    variant.sources.jniLibs?.addGeneratedSourceDirectory(cargoNdkBuild, CargoNdkBuild::outputDir)
    variant.sources.kotlin?.addGeneratedSourceDirectory(uniffiBindgen, UniffiBindgen::outputDir)
}

useHostRustInTests()

dependencies {
    // UniFFI's generated Kotlin calls the Rust library through JNA; the AAR carries JNA's
    // Android native dispatcher, the JAR the desktop one for JVM unit tests.
    api("${libs.jna.get()}@aar")
    testImplementation(libs.jna)
    testImplementation(libs.junit)
}
