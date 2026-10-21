package io.github.locadani.whichride.ui

import androidx.compose.ui.test.assertIsOff
import androidx.compose.ui.test.assertIsOn
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextReplacement
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.personal.AmountOff
import io.github.locadani.whichride.personal.DealField
import io.github.locadani.whichride.personal.DealForm
import io.github.locadani.whichride.personal.DealKind
import io.github.locadani.whichride.personal.PercentOff
import io.github.locadani.whichride.personal.Personal
import java.time.LocalDate
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class DealsScreenTest {
    @get:Rule val rule = createComposeRule()
    private val today = LocalDate.of(2026, 10, 20).toEpochDay()
    private val deals = listOf(
        PercentOff("a", "bolt", 2_000, until = today + 11),
        AmountOff("b", "miles", 500, once = true, until = null, enabled = false),
        PercentOff("c", "bolt", 1_000, until = today - 1),
    )
    private val state = UiState(stack = listOf(Screen.Home, Screen.Settings, Screen.Deals), providers = PROVIDERS, personal = Personal(deals), today = today)

    @Test fun `deals show their state and switch`() {
        val toggled = mutableListOf<String>()
        rule.setContent { WhichrideTheme { DealsScreen(state, {}, {}, {}, { toggled += it }) } }
        rule.onNodeWithText("Bolt Drive −20%").assertExists()
        rule.onNodeWithText("once · next trip only").assertExists()
        rule.onNodeWithText("expired").assertExists()
        rule.onNodeWithContentDescription("Bolt Drive −20% on").assertIsOn()
        rule.onNodeWithContentDescription("MILES −€5.00 on").assertIsOff().performClick()
        assertEquals(listOf("b"), toggled)
    }

    @Test fun `the editor shows fields for the kind and marks what to fix`() {
        var form = DealForm(providerId = null, kind = DealKind.PASS, invalid = setOf(DealField.PROVIDER, DealField.AMOUNT))
        rule.setContent {
            WhichrideTheme { DealEditorScreen(state.copy(dealForm = form), {}, { form = it }, {}, {}) }
        }
        rule.onNodeWithText("Choose a provider").assertExists()
        rule.onNodeWithText("Enter at least one saving").assertExists()
        rule.onNodeWithText("Off per minute (%)").assertExists()
        rule.onNodeWithText("Valid until").assertDoesNotExist()
        rule.onNodeWithText("Bolt Drive").performClick()
        assertEquals("bolt", form.providerId)
        rule.onNodeWithText("Off per km (%)").performTextReplacement("10")
        assertEquals("10", form.amount)
    }
}
