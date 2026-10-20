package io.github.locadani.whichride.ui

import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.AppInfo
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.designsystem.WhichrideTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SettingsScreenTest {
    @get:Rule val rule = createComposeRule()
    private val sixt = ProviderInfo("sixt", "SIXT share", "com.trafi.whitelabel.bvg", emptyList(),
        listOf(AppInfo("BVG Jelbi", "com.trafi.whitelabel.bvg"), AppInfo("SIXT", "com.sixt.reservation")))

    @Test fun `only providers with a choice of apps are listed, the current one selected`() {
        val chosen = mutableListOf<Pair<String, String>>()
        rule.setContent {
            WhichrideTheme { SettingsScreen(UiState(providers = PROVIDERS + sixt), {}, {}, { p, a -> chosen += p to a }, {}, {}) }
        }
        rule.onNodeWithText("Open SIXT share with").assertExists()
        rule.onNodeWithText("Open MILES with").assertDoesNotExist()
        rule.onNodeWithText("BVG Jelbi").assertIsSelected()
        rule.onNodeWithText("SIXT").performClick()
        assertEquals(listOf("sixt" to "com.sixt.reservation"), chosen)
    }
}
