package io.github.locadani.whichride.launch

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Test

/** Every provider app in the seed catalogue is visible to "Open <provider>" (decision 62). */
class ManifestQueriesTest {
    @Test
    fun theManifestQueriesEveryCatalogueProvider() {
        val catalogue = File("../../crates/server/catalogue.json").readText()
        val manifest = File("src/main/AndroidManifest.xml").readText()
        val packages = Regex(""""android_package"\s*:\s*"([^"]+)"""").findAll(catalogue).map { it.groupValues[1] }.toSet()
        val queried = Regex("""<package android:name="([^"]+)"""").findAll(manifest).map { it.groupValues[1] }.toSet()
        assertEquals(packages, queried)
    }
}
