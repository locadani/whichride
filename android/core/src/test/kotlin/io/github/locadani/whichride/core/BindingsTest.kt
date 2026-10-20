package io.github.locadani.whichride.core

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Calls the real Rust library (host build) through the generated bindings. */
class BindingsTest {
    private val seed = File("../../crates/server/catalogue.json").readText()
    private val trip = Trip(metres = 7_400u, seconds = 1_320u)

    @Test
    fun parsesAShortLinkAndItsLabel() {
        val text = "Markthalle Neun\nhttps://maps.app.goo.gl/x7Qa2Lm"
        assertEquals(Destination.ShortLink("https://maps.app.goo.gl/x7Qa2Lm"), parseDestination(text))
        assertEquals("Markthalle Neun", sharedLabel(text))
    }

    @Test(expected = LinkException.NotGoogleMaps::class)
    fun linkErrorsAreExceptions() {
        parseDestination("https://example.com")
    }

    @Test
    fun pricesTheSeedCatalogue() {
        val prices = Prices.fromSeed(seed)
        assertEquals(listOf("miles", "bolt", "sixt"), prices.providers().map { it.id })
        val quotes = prices.quotes(trip, 1_792_497_600L)
        assertEquals(9, quotes.size)
        assertTrue(quotes.zipWithNext().all { (a, b) -> a.totalCents <= b.totalCents })
        assertNull(quotes.first().confirmedDaysAgo)
    }

    @Test
    fun ranksOptions() {
        val prices = Prices.fromSeed(seed)
        val options = prices.options(trip, listOf(Choice.AnyOf("bolt")), 1_792_497_600L)
        assertEquals("bolt", options.ranked.single().quote.providerId)
    }

    @Test(expected = DataException::class)
    fun refusesABrokenCatalogue() {
        Prices.fromSeed("{")
    }
}
