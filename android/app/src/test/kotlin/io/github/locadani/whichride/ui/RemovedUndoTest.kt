package io.github.locadani.whichride.ui

import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.remember
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.Removed
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.designsystem.WhichrideTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class RemovedUndoTest {
    @get:Rule val compose = createComposeRule()
    private val removed = Removed(RankedOption(Choice.Offer("bolt.polo"), QUOTES[2]), wasSelected = false)

    private fun show(onUndo: () -> Unit, onShown: () -> Unit) = compose.setContent {
        WhichrideTheme {
            val snackbar = remember { SnackbarHostState() }
            RemovedUndo(UiState(providers = PROVIDERS, removed = removed), snackbar, onUndo, onShown)
            SnackbarHost(snackbar)
        }
    }

    @Test fun `undo puts the removed car back`() {
        var undone = 0
        show(onUndo = { undone++ }, onShown = {})
        compose.onNodeWithText("Bolt Drive VW Polo removed").assertExists()
        compose.onNodeWithText("Undo").performClick()
        compose.waitForIdle()
        assertEquals(1, undone)
    }

    @Test fun `the message goes away on its own`() {
        var shown = 0
        compose.mainClock.autoAdvance = false
        show(onUndo = {}, onShown = { shown++ })
        compose.mainClock.advanceTimeBy(20_000)
        assertEquals(1, shown)
    }
}
