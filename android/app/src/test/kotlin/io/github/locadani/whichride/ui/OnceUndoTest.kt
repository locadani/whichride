package io.github.locadani.whichride.ui

import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.runtime.remember
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.personal.AmountOff
import io.github.locadani.whichride.personal.Personal
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/** Review finding I1: the user is in the provider's app while the Undo would time out. */
@RunWith(AndroidJUnit4::class)
class OnceUndoTest {
    @get:Rule val compose = createComposeRule()

    @Test fun `undo waits for the user to come back from the provider's app`() {
        var undone = 0
        val state = UiState(
            providers = PROVIDERS,
            personal = Personal(deals = listOf(AmountOff("m", "miles", 500, once = true, until = null, used = true))),
            usedOnce = listOf("m"),
        )
        compose.mainClock.autoAdvance = false
        compose.setContent {
            WhichrideTheme {
                val snackbar = remember { SnackbarHostState() }
                OnceUndo(state, snackbar, onUndo = { undone++ }, onShown = {})
                SnackbarHost(snackbar)
            }
        }
        compose.mainClock.advanceTimeBy(60_000)
        compose.onNodeWithText("Undo").performClick()
        compose.mainClock.advanceTimeBy(1_000)
        assertEquals(1, undone)
    }
}
