import java.io.File
import java.util.Properties
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.provider.ProviderFactory
import org.gradle.api.provider.SetProperty
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.TaskAction

/**
 * whichride's per-machine settings (spec 2026-10-21 §3.4): android/local.properties first, then
 * a Gradle property (-P, ~/.gradle/gradle.properties), then WHICHRIDE_<NAME>. Blank means unset.
 */
class LocalSettings(private val providers: ProviderFactory, localFile: File) {
    private val local = Properties().apply { if (localFile.isFile) localFile.inputStream().use(::load) }
    private val asked = sortedSetOf<String>()

    /** Every setting the build read, for [CheckLocalSettings]. */
    val names: Set<String> get() = asked

    fun get(name: String): String? {
        require(name.startsWith("whichride.")) { "whichride settings start with whichride.: $name" }
        asked += name
        val value = local.getProperty(name)
            ?: providers.gradleProperty(name).orNull
            ?: providers.environmentVariable(envName(name)).orNull
        return value?.trim()?.takeIf { it.isNotEmpty() }
    }

    companion object {
        /** whichride.upload.storeFile → WHICHRIDE_UPLOAD_STOREFILE (the release workflow's names). */
        fun envName(name: String) = "WHICHRIDE_" + name.removePrefix("whichride.").replace('.', '_').uppercase()
    }
}

/** Fails when the build reads a setting the template doesn't list, or the template lists one it never reads. */
abstract class CheckLocalSettings : DefaultTask() {
    @get:Input abstract val names: SetProperty<String>
    @get:InputFile abstract val template: RegularFileProperty

    @TaskAction
    fun check() {
        val listed = Properties().apply { template.get().asFile.inputStream().use(::load) }.stringPropertyNames()
        val used = names.get()
        val unlisted = used - listed
        val unused = listed - used
        if (unlisted.isNotEmpty() || unused.isNotEmpty()) {
            throw GradleException(
                "android/local.properties.example is out of date. Add: $unlisted. Remove: $unused.",
            )
        }
    }
}
