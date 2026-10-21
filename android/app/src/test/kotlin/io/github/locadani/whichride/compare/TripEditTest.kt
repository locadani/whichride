package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Trip
import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Review finding: "7.449 km" reads as thousands of km in German, Italian and Turkish. */
class TripEditTest {
    private val routed = Trip(7_449u, 1_330u)

    @Test fun kmAreShownLikeTheTitleLineInTheUsersLocale() {
        assertEquals("7,4", TripEdit(routed, Locale.GERMANY).km)
        assertEquals("7.4", TripEdit(routed, Locale.UK).km)
        assertEquals("7,4", TripEdit(routed, Locale.ITALY).originalKm)
    }

    @Test fun theShownValueIsNotAnEdit() {
        assertFalse(TripEdit(routed, Locale.GERMANY).kmEdited)
        assertFalse(TripEdit(routed, Locale.GERMANY, km = "7.4").kmEdited)
        assertTrue(TripEdit(routed, Locale.GERMANY, km = "7,5").kmEdited)
    }
}
