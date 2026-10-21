import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class LocalSettingsTest {
    @Test fun aBlankLineInLocalPropertiesFallsThrough() {
        assertEquals("https://server", LocalSettings.firstSet({ "" }, { "https://server" }, { null }))
        assertEquals("from-env", LocalSettings.firstSet({ "  " }, { null }, { "from-env" }))
    }

    @Test fun theFirstValueWinsTrimmed() {
        assertEquals("key", LocalSettings.firstSet({ " key\n" }, { "other" }, { error("not read") }))
    }

    @Test fun allBlankIsUnset() {
        assertNull(LocalSettings.firstSet({ "" }, { " " }, { null }))
    }

    @Test fun envNamesMatchTheReleaseWorkflow() {
        assertEquals("WHICHRIDE_UPLOAD_STOREFILE", LocalSettings.envName("whichride.upload.storeFile"))
        assertEquals("WHICHRIDE_REGISTRATIONKEY", LocalSettings.envName("whichride.registrationKey"))
    }
}
