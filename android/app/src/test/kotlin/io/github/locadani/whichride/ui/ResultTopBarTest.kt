package io.github.locadani.whichride.ui

import androidx.compose.ui.test.getBoundsInRoot
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.unit.dp
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.TripEdit
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.designsystem.WhichrideTheme
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

/** The phone in Italian squeezed the trip line into one letter per row (milestone 6 phone test). */
@RunWith(AndroidJUnit4::class)
@Config(qualifiers = "it-w393dp-h851dp")
@GraphicsMode(GraphicsMode.Mode.NATIVE) // real text widths; the default mode measures text as nearly zero
class ResultTopBarTest {
    @get:Rule val compose = createComposeRule()

    @Test fun theTripLineKeepsItsWidthAndTheChevronStaysBesideIt() {
        compose.setContent { WhichrideTheme { ResultScreen(resultState(), resultActions()) } }
        val line = compose.onNodeWithText("7,4 km", substring = true, useUnmergedTree = true).getBoundsInRoot()
        val chevron = compose.onNodeWithContentDescription("Mostra i dettagli del viaggio", useUnmergedTree = true).getBoundsInRoot()
        assertTrue("trip line is ${line.right - line.left} wide", line.right - line.left > 80.dp)
        assertTrue("the chevron (${chevron.left}..${chevron.right}) overlaps the trip line (${line.left}..${line.right})", chevron.left >= line.right)
    }

    @Test fun distanceAndTimeSitSideBySide() {
        compose.setContent { WhichrideTheme { ResultScreen(resultState().copy(tripEdit = TripEdit(Trip(7_400u, 1_320u))), resultActions()) } }
        val distance = compose.onNodeWithText("7,4").getBoundsInRoot()
        val time = compose.onNodeWithText("22").getBoundsInRoot()
        assertTrue("time (${time.left}) is not right of distance (${distance.right})", time.left >= distance.right)
        assertTrue("not on one row", time.top < distance.bottom && distance.top < time.bottom)
    }
}
