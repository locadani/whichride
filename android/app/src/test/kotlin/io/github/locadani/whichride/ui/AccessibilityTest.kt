package io.github.locadani.whichride.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.unit.Density
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.QuestionState
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.Question
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.crowd.PricePurpose
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.personal.DealForm
import io.github.locadani.whichride.personal.DealKind
import io.github.locadani.whichride.personal.PercentOff
import io.github.locadani.whichride.personal.Personal
import io.github.locadani.whichride.personal.RefuelCreditDeal
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.Parameterized
import org.robolectric.ParameterizedRobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(ParameterizedRobolectricTestRunner::class)
@Config(qualifiers = "w360dp-h640dp")
class AccessibilityTest(private val fontScale: Float) {
    companion object {
        @JvmStatic @ParameterizedRobolectricTestRunner.Parameters(name = "font scale {0}")
        fun scales() = listOf(arrayOf<Any>(1f), arrayOf<Any>(2f))
    }

    @get:Rule val rule = createComposeRule()

    private fun show(content: @Composable () -> Unit) = rule.setContent {
        val density = LocalDensity.current
        CompositionLocalProvider(LocalDensity provides Density(density.density, fontScale)) { WhichrideTheme { content() } }
    }

    private val options = listOf(
        RankedOption(Choice.AnyOf("bolt"), QUOTES[0]),
        RankedOption(Choice.Offer("miles.s"), QUOTES[1]),
    )
    private val personal = Personal(deals = listOf(PercentOff("a", "bolt", 2_000, until = null), RefuelCreditDeal("f", "miles", 300)))

    @Test fun result() {
        show { ResultScreen(resultState(options).copy(personal = personal), ResultActions()) }
        rule.assertAccessible()
        rule.onNodeWithText("Open Bolt Drive").assertIsDisplayed()
    }

    @Test fun emptyResult() {
        show { ResultScreen(resultState(), ResultActions()) }
        rule.assertAccessible()
        rule.onNodeWithText("Add the cars near you").assertIsDisplayed()
    }

    @Test fun cars() {
        show { CarsScreen(resultState().copy(stack = listOf(Screen.Home, Screen.Cars("bolt")), crowdEnabled = true), "bolt", {}, {}, {}, {}, {}, {}) }
        rule.assertAccessible()
        rule.onNodeWithText("Done").assertIsDisplayed()
    }

    @Test fun providers() {
        show { ProvidersScreen(resultState(), {}, {}) }
        rule.assertAccessible()
    }

    @Test fun deals() {
        show { DealsScreen(resultState().copy(personal = personal), {}, {}, {}, {}) }
        rule.assertAccessible()
    }

    @Test fun dealEditor() {
        show { DealEditorScreen(resultState().copy(dealForm = DealForm(kind = DealKind.PASS)), {}, {}, {}, {}) }
        rule.assertAccessible()
        rule.onNodeWithText("Save discount").assertIsDisplayed()
    }

    @Test fun settings() {
        show { SettingsScreen(resultState(), {}, {}, { _, _ -> }, {}, {}) }
        rule.assertAccessible()
    }

    @Test fun priceForm() {
        show { PriceFormScreen(resultState().copy(priceForm = PriceForm(PricePurpose.Propose("bolt"))), {}, {}, {}, {}) }
        rule.assertAccessible()
        rule.onNodeWithText("Send car").assertIsDisplayed()
    }

    @Test fun question() {
        val asked = QuestionState(Question("bolt.polo", null, emptyList()), "bolt", "VW Polo")
        show { QuestionContent(resultState().copy(question = asked), QuestionActions()) }
        rule.assertAccessible()
    }

    @Test fun home() {
        show { HomeScreen(resultState().copy(stack = listOf(Screen.Home)), {}, {}, {}, {}, {}) }
        rule.assertAccessible()
    }

    @Test fun manualTrip() {
        show { ManualTripScreen(resultState().copy(stack = listOf(Screen.Home, Screen.ManualTrip)), {}, {}, {}, {}, {}, {}) }
        rule.assertAccessible()
        rule.onNodeWithText("Compare").assertIsDisplayed()
    }
}
