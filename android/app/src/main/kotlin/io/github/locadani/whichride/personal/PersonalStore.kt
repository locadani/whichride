package io.github.locadani.whichride.personal

import java.io.File
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json

/** Where [Personal] lives between launches; the ViewModel's tests fake it. */
interface PersonalStore {
    fun load(): Personal
    fun save(personal: Personal)
}

/**
 * One JSON file in app storage (plan R4, decision 67). Backups are off for the whole app, so it
 * never leaves the phone. An unreadable file starts empty rather than stopping the comparison.
 */
class FilePersonalStore(private val file: File) : PersonalStore {
    private val json = Json { ignoreUnknownKeys = true }

    override fun load(): Personal = try {
        if (file.exists()) json.decodeFromString<Personal>(file.readText()) else Personal()
    } catch (_: SerializationException) {
        Personal()
    } catch (_: IllegalArgumentException) {
        Personal()
    }

    override fun save(personal: Personal) {
        val temporary = File(file.parentFile, "${file.name}.tmp")
        temporary.writeText(json.encodeToString(personal))
        temporary.renameTo(file)
    }
}
