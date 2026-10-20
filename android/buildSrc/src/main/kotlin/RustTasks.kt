import javax.inject.Inject
import org.gradle.api.DefaultTask
import org.gradle.api.Project
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.provider.ListProperty
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import org.gradle.api.tasks.testing.Test
import org.gradle.process.ExecOperations

/** The repository root: the Cargo workspace that holds `android/`. */
val Project.repoRoot get() = rootProject.layout.projectDirectory.dir("..")

/** Everything that changes the Rust library: the inputs of every cargo task. */
fun Project.rustSources(): ConfigurableFileCollection = files(
    repoRoot.file("Cargo.toml"),
    repoRoot.file("Cargo.lock"),
    repoRoot.file("rust-toolchain.toml"),
    repoRoot.dir("crates/core"),
    repoRoot.dir("crates/ffi"),
    repoRoot.dir("crates/uniffi-bindgen"),
)

/** Runs cargo in the workspace root. */
abstract class CargoTask @Inject constructor(private val exec: ExecOperations) : DefaultTask() {
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection

    @get:Internal
    abstract val workspace: DirectoryProperty

    protected fun cargo(arguments: List<String>, environment: Map<String, String> = emptyMap()) {
        exec.exec {
            workingDir = workspace.get().asFile
            environment(environment)
            commandLine(listOf("cargo") + arguments)
        }
    }
}

/** `cargo ndk`: the release library for each Android ABI, laid out as `jniLibs/<abi>/`. */
abstract class CargoNdkBuild @Inject constructor(exec: ExecOperations) : CargoTask(exec) {
    @get:Input abstract val abis: ListProperty<String>
    @get:Input abstract val minSdk: org.gradle.api.provider.Property<Int>
    @get:Internal abstract val ndkDirectory: DirectoryProperty
    @get:OutputDirectory abstract val outputDir: DirectoryProperty

    @TaskAction
    fun build() {
        val targets = abis.get().flatMap { listOf("-t", it) }
        cargo(
            listOf("ndk") + targets + listOf(
                "-P", minSdk.get().toString(),
                "-o", outputDir.get().asFile.absolutePath,
                "build", "--locked", "-p", "whichride-ffi", "--profile", "android",
            ),
            mapOf("ANDROID_NDK_HOME" to ndkDirectory.get().asFile.absolutePath),
        )
    }
}

/** Kotlin bindings read from a built library (UniFFI library mode). */
abstract class UniffiBindgen @Inject constructor(exec: ExecOperations) : CargoTask(exec) {
    @get:InputFile @get:PathSensitive(PathSensitivity.NONE)
    abstract val library: RegularFileProperty
    @get:OutputDirectory abstract val outputDir: DirectoryProperty

    @TaskAction
    fun generate() {
        outputDir.get().asFile.deleteRecursively()
        cargo(
            listOf(
                "run", "--locked", "-p", "uniffi-bindgen", "--", "generate",
                "--library", library.get().asFile.absolutePath,
                "--language", "kotlin", "--no-format",
                "--out-dir", outputDir.get().asFile.absolutePath,
            ),
        )
    }
}

/**
 * A debug build for the machine running Gradle: JVM unit tests call the real Rust through it,
 * and the bindings are generated from it (the stripped Android library has no UniFFI metadata).
 */
abstract class CargoHostBuild @Inject constructor(exec: ExecOperations) : CargoTask(exec) {
    @get:OutputFile abstract val library: RegularFileProperty

    @TaskAction
    fun build() = cargo(listOf("build", "--locked", "-p", "whichride-ffi"))
}

/** The host library's file name: `libwhichride_ffi.dylib` on macOS, `.so` on Linux. */
val hostLibraryName: String
    get() = if (System.getProperty("os.name").startsWith("Mac")) "libwhichride_ffi.dylib" else "libwhichride_ffi.so"

/** JVM unit tests of this module load the host build through JNA. */
fun Project.useHostRustInTests() {
    tasks.withType(Test::class.java).configureEach {
        dependsOn(":core:cargoHostBuild")
        systemProperty("jna.library.path", repoRoot.dir("target/debug").asFile.absolutePath)
    }
}
