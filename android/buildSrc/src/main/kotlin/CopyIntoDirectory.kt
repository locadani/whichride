import org.gradle.api.DefaultTask
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.MapProperty
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction

/**
 * Copies files from elsewhere in the repository into a generated source directory, under the
 * names in [targets] (source file name → path inside [outputDir]), so the APK carries them
 * without a committed copy.
 */
abstract class CopyIntoDirectory : DefaultTask() {
    @get:InputFiles @get:PathSensitive(PathSensitivity.NAME_ONLY)
    abstract val sources: ConfigurableFileCollection
    @get:Input abstract val targets: MapProperty<String, String>
    @get:OutputDirectory abstract val outputDir: DirectoryProperty

    @TaskAction
    fun copy() {
        val out = outputDir.get().asFile
        out.deleteRecursively()
        val names = targets.get()
        sources.files.forEach { file ->
            val target = out.resolve(names[file.name] ?: error("no target for ${file.name}"))
            target.parentFile.mkdirs()
            file.copyTo(target, overwrite = true)
        }
    }
}
