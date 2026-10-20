package io.github.locadani.whichride.ui

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.OfferInfo
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.designsystem.WhichrideTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class CarsScreenTest {
    @get:Rule val compose = createComposeRule()
    private val toggled = mutableListOf<Choice>()

    private val models = listOf("Kia Picanto", "VW Polo", "Toyota Yaris", "Opel Corsa", "Skoda Fabia", "Seat Ibiza", "Renault Clio")

    private fun show(offers: Int, draft: Set<Choice> = emptySet(), filter: String = "") {
        val names = models.take(offers)
        val bolt = ProviderInfo("bolt", "Bolt Drive", "ee.mtakso.client", names.map { OfferInfo("bolt.${it.lowercase()}", it) })
        val quotes = names.mapIndexed { i, name -> quote("bolt", "bolt.${name.lowercase()}", name, 790L + i * 10) }
        val state = resultState().copy(stack = listOf(Screen.Home, Screen.Result, Screen.Providers, Screen.Cars("bolt")),
            providers = listOf(bolt), quotes = quotes, draft = draft, carFilter = filter)
        compose.setContent {
            WhichrideTheme {
                CarsScreen(state, "bolt", onBack = {}, onCheckIn = {}, onToggle = { toggled += it }, onFilter = {}, onCommit = {})
            }
        }
    }

    @Test fun noSearchBoxForSixCarsOrFewer() {
        show(offers = 6)
        compose.onNodeWithText("Filter 6 cars, e.g. polo").assertDoesNotExist()
    }

    @Test fun aSearchBoxForMoreThanSixCars() {
        show(offers = 7)
        compose.onNodeWithText("Filter 7 cars, e.g. polo").assertExists()
    }

    @Test fun anyCarComesFirstAndTogglesItsChoice() {
        show(offers = 3)
        compose.onNodeWithText("Any Bolt Drive car").performClick()
        assertEquals(listOf<Choice>(Choice.AnyOf("bolt")), toggled)
        compose.onNodeWithText("from €7.90").assertExists()
        compose.onNodeWithText("Done").assertExists()
    }

    @Test fun theButtonCountsTheTickedCars() {
        show(offers = 3, draft = setOf(Choice.Offer("bolt.vw polo"), Choice.Offer("bolt.kia picanto")))
        compose.onNodeWithText("Add 2 cars").assertExists()
    }

    @Test fun anyCarSaysSo() {
        show(offers = 3, draft = setOf(Choice.AnyOf("bolt")))
        compose.onNodeWithText("Add any Bolt Drive car").assertExists()
    }

    @Test fun filteringHidesAnyCarAndOtherModels() {
        show(offers = 7, filter = "polo")
        compose.onNodeWithText("VW Polo").assertExists()
        compose.onNodeWithText("Kia Picanto").assertDoesNotExist()
        compose.onNodeWithText("Any Bolt Drive car").assertDoesNotExist()
    }

    @Test fun aFilterWithoutMatchesSaysSo() {
        show(offers = 7, filter = "tesla")
        compose.onNodeWithText("No Bolt Drive car matches “tesla”.").assertExists()
    }
}
