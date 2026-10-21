import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class CheckLocalSettingsTest {
    @Test fun inSyncIsFine() {
        assertNull(templateProblem(used = setOf("whichride.a", "whichride.b"), listed = setOf("whichride.a", "whichride.b")))
    }

    @Test fun aSettingMissingFromTheTemplateFails() {
        assertEquals(
            "android/local.properties.example is out of date. Add: [whichride.b]. Remove: [].",
            templateProblem(used = setOf("whichride.a", "whichride.b"), listed = setOf("whichride.a")),
        )
    }

    @Test fun aTemplateLineNobodyReadsFails() {
        assertEquals(
            "android/local.properties.example is out of date. Add: []. Remove: [whichride.old].",
            templateProblem(used = setOf("whichride.a"), listed = setOf("whichride.a", "whichride.old")),
        )
    }

    @Test fun settingNamesAreFoundInBuildScripts() {
        val script = """
            val a = settings.get("whichride.serverUrl")
            val b = providers.gradleProperty("whichride.upload.storeFile").orNull
            require(x) { "whichride.registrationKey must be at least 32 characters" }
            namespace = "io.github.locadani.whichride"
            require(name.startsWith("whichride.")) { "whichride settings start with whichride.: ${'$'}name" }
            // whichride.notAString in a comment
        """
        assertEquals(
            setOf("whichride.serverUrl", "whichride.upload.storeFile", "whichride.registrationKey"),
            settingNamesIn(script),
        )
    }
}
