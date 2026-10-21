package io.github.locadani.whichride.ui

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.crowd.PriceField
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.crowd.PricePurpose
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.core.Options
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.core.Choice
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class PriceFormScreenTest {
    @get:Rule val rule = createComposeRule()

    @Test fun `a report marks out-of-range values and offers the shared price back`() {
        var reset: String? = null
        val mine = quote("bolt", "bolt.polo", "VW Polo", 912).copy(overridden = true)
        val state = UiState(
            stack = listOf(Screen.Home, Screen.Result, Screen.PriceForm),
            providers = PROVIDERS,
            quotes = listOf(mine),
            priceForm = PriceForm(PricePurpose.Report("bolt.polo", "bolt", "VW Polo"), perKm = "9", invalid = setOf(PriceField.PER_KM)),
        )
        rule.setContent { WhichrideTheme { PriceFormScreen(state, {}, {}, {}, { reset = it }) } }
        rule.onNodeWithText("What does Bolt Drive show?").assertExists()
        rule.onNodeWithText("Up to €5.00, e.g. 0,29").assertExists()
        rule.onNodeWithText("Use the shared price").performScrollTo().performClick()
        assertEquals("bolt.polo", reset)
    }

    @Test fun `a proposal asks for the car's name`() {
        val state = UiState(providers = PROVIDERS, priceForm = PriceForm(PricePurpose.Propose("bolt")))
        rule.setContent { WhichrideTheme { PriceFormScreen(state, {}, {}, {}, {}) } }
        rule.onNodeWithText("Add a Bolt Drive car").assertExists()
        rule.onNodeWithText("Car name").assertExists()
        rule.onNodeWithText("Send car").assertExists()
        rule.onNodeWithText("Use the shared price").assertDoesNotExist()
    }
}
