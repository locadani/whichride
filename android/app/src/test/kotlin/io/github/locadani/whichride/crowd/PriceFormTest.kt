package io.github.locadani.whichride.crowd

import io.github.locadani.whichride.core.CardKind
import io.github.locadani.whichride.core.RateCard
import io.github.locadani.whichride.core.Rounding
import java.util.Locale
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PriceFormTest {
    private val standard = RateCard("Standard", CardKind.STANDARD, 100, 29, 11, 0u, 0u, 250, Rounding.STARTED_UNIT, Rounding.STARTED_UNIT)
    private val pack = RateCard("3 h", CardKind.PACKAGE, 2_900, 19, 0, 40u, 180u, 0, Rounding.STARTED_UNIT, Rounding.STARTED_UNIT)
    private val report = PricePurpose.Report("bolt.polo", "bolt", "VW Polo")

    @Test fun `the form is prefilled from the standard card`() {
        val form = priceForm(report, listOf(pack, standard), Locale.GERMANY)
        assertEquals(listOf("1", "0,29", "0,11", "2,50"), listOf(form.unlock, form.perKm, form.perMinute, form.minimum))
    }

    @Test fun `a correction replaces only the standard card's values`() {
        val form = priceForm(report, listOf(standard, pack), Locale.GERMANY).copy(perKm = "0,31")
        val built = form.build(listOf(standard, pack), null) as Built.Report
        assertEquals(listOf(standard.copy(perKm = 31), pack), built.cards)
        val body = Json.parseToJsonElement(built.body).jsonObject
        assertEquals("bolt.polo", body["offer"]!!.jsonPrimitive.content)
        assertEquals(2, body["cards"]!!.jsonArray.size)
    }

    @Test fun `unchanged values send nothing`() {
        val form = priceForm(report, listOf(standard), Locale.UK)
        assertEquals(Built.Unchanged, form.build(listOf(standard), null))
    }

    @Test fun `values above the limits are refused before sending`() {
        val form = priceForm(report, listOf(standard), Locale.GERMANY).copy(perKm = "5,01", minimum = "x", unlock = "501")
        assertEquals(Built.Invalid(setOf(PriceField.PER_KM, PriceField.MINIMUM, PriceField.UNLOCK)), form.build(listOf(standard), null))
    }

    @Test fun `a proposal copies rounding and label from the provider's template`() {
        val template = standard.copy(minuteRounding = Rounding.EXACT)
        val form = PriceForm(PricePurpose.Propose("bolt"), name = " Fiat 500e ", unlock = "1", perKm = "0,25", perMinute = "0,10", minimum = "")
        val built = form.build(emptyList(), template) as Built.Proposal
        val body = Json.parseToJsonElement(built.body).jsonObject
        assertEquals("Fiat 500e", body["name"]!!.jsonPrimitive.content)
        val card = body["cards"]!!.jsonArray.single().jsonObject
        assertEquals("exact", card["minute_rounding"]!!.jsonPrimitive.content)
        assertEquals("0", card["minimum"]!!.jsonPrimitive.content)
        assertEquals(Built.Invalid(setOf(PriceField.NAME)), form.copy(name = " ").build(emptyList(), template))
    }

    @Test fun `the prefix matches the core's`() {
        assertEquals(io.github.locadani.whichride.core.proposalPrefix(), PROPOSAL_PREFIX)
        assertTrue(PROPOSAL_PREFIX.endsWith("."))
    }
}
