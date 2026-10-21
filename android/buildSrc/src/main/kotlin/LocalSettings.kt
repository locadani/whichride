import java.io.File
import java.util.Properties
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.provider.ProviderFactory
import org.gradle.api.provider.SetProperty
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
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
        return firstSet(
            { local.getProperty(name) },
            { providers.gradleProperty(name).orNull },
            { providers.environmentVariable(envName(name)).orNull },
        )
    }

    companion object {
        /** The first non-blank value, trimmed: a blank line copied from the template doesn't hide the others. */
        fun firstSet(vararg sources: () -> String?): String? =
            sources.asSequence().map { it()?.trim() }.firstOrNull { !it.isNullOrEmpty() }

        /** whichride.upload.storeFile → WHICHRIDE_UPLOAD_STOREFILE (the release workflow's names). */
        fun envName(name: String) = "WHICHRIDE_" + name.removePrefix("whichride.").replace('.', '_').uppercase()
    }
}

/**
 * Fails when the build uses a setting the template doesn't list, or the template lists one it
 * never uses. "Used" is every name read through [LocalSettings] plus every "whichride.…" string
 * literal in the build scripts, so a read that skips [LocalSettings] is caught too.
 */
abstract class CheckLocalSettings : DefaultTask() {
    @get:Input abstract val names: SetProperty<String>
    @get:InputFile @get:PathSensitive(PathSensitivity.RELATIVE) abstract val template: RegularFileProperty
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE) abstract val buildScripts: ConfigurableFileCollection

    @TaskAction
    fun check() {
        val listed = Properties().apply { template.get().asFile.inputStream().use(::load) }.stringPropertyNames()
        val used = names.get() + buildScripts.files.flatMap { settingNamesIn(it.readText()) }
        templateProblem(used, listed)?.let { throw GradleException(it) }
    }
}

/** What is wrong between the settings the build uses and the ones the template lists, or null. */
fun templateProblem(used: Set<String>, listed: Set<String>): String? {
    val unlisted = (used - listed).sorted()
    val unused = (listed - used).sorted()
    if (unlisted.isEmpty() && unused.isEmpty()) return null
    return "android/local.properties.example is out of date. Add: $unlisted. Remove: $unused."
}

private val settingLiteral = Regex(""""(whichride\.[A-Za-z][A-Za-z0-9_.]*[A-Za-z0-9])""")

/** Setting names in string literals: `"whichride.serverUrl"`, or a message starting with one. */
fun settingNamesIn(source: String): Set<String> =
    settingLiteral.findAll(source).map { it.groupValues[1] }.toSet()
