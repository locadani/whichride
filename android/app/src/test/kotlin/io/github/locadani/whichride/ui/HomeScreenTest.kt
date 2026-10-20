package io.github.locadani.whichride.ui

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.InputError
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.server.Place
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class HomeScreenTest {
    @get:Rule val compose = createComposeRule()
    private var submitted = 0
    private val picked = mutableListOf<Place>()

    private fun show(state: UiState) = compose.setContent {
        WhichrideTheme {
            HomeScreen(state, onQuery = {}, onSubmit = { submitted++ }, onSuggestion = { picked += it }, onChangeStart = {}, onLicences = {})
        }
    }

    @Test fun aPastedLinkIsOfferedForComparison() {
        show(UiState(query = "https://maps.app.goo.gl/x", linkInQuery = true))
        compose.onNodeWithText("Compare this trip").performClick()
        assertEquals(1, submitted)
    }

    @Test fun suggestionsShowNameAndArea() {
        val place = Place("Markthalle Neun, Eisenbahnstraße 42, Berlin", Coordinate(525_022_870, 134_316_050))
        show(UiState(query = "Markth", suggestions = listOf(place)))
        compose.onNodeWithText("Eisenbahnstraße 42, Berlin").assertExists()
        compose.onNodeWithText("Markthalle Neun").performClick()
        assertEquals(listOf(place), picked)
    }

    @Test fun inputErrorsAreShown() {
        show(UiState(inputError = InputError.NOT_A_MAPS_LINK))
        compose.onNodeWithText("That link isn't from Google Maps.").assertExists()
    }

    @Test fun theTipShowsWhenIdle() {
        show(UiState())
        compose.onNodeWithText("Tip: in Google Maps, tap Share on a place and pick whichride.").assertExists()
    }
}
