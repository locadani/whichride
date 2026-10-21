package io.github.locadani.whichride.personal

import io.github.locadani.whichride.core.AppInfo
import io.github.locadani.whichride.core.CardKind
import io.github.locadani.whichride.core.Discount
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.core.RateCard
import io.github.locadani.whichride.core.Rounding
import java.time.LocalDate
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

fun card(perKm: Long) = RateCard("Standard", CardKind.STANDARD, 100, perKm, 0, 0u, 0u, 0, Rounding.STARTED_UNIT, Rounding.STARTED_UNIT)

class PersonalTest {
    private val oct31 = LocalDate.of(2026, 10, 31).toEpochDay()
    private val bolt20 = PercentOff("a", "bolt", 2_000, until = oct31, enabled = true)
    private val milesOnce = AmountOff("b", "miles", 500, once = true, until = null, used = false, enabled = true)
    private val milesPass = Pass("c", "miles", distanceOff = 1_000, timeOff = 0, packageOff = 0, waiveUnlock = true, enabled = true)
    private val milesFuel = RefuelCreditDeal("d", "miles", 300, enabled = true)
    private val personal = Personal(deals = listOf(bolt20, milesOnce, milesPass, milesFuel))

    @Test fun `a discount applies through its last day`() {
        assertTrue(bolt20.isActive(oct31))
        assertFalse(bolt20.isActive(oct31 + 1))
        assertFalse(bolt20.copy(enabled = false).isActive(oct31))
        assertFalse(milesOnce.copy(used = true).isActive(oct31))
    }

    @Test fun `only active deals reach the core`() {
        val adjustments = personal.adjustments(today = oct31 + 1, refuel = false)
        assertEquals(listOf(Discount.Amount("miles", 500)), adjustments.discounts)
        assertEquals(1, adjustments.subscriptions.size)
        assertEquals(1_000.toUShort(), adjustments.subscriptions[0].distanceOff)
        assertTrue(adjustments.subscriptions[0].waiveBaseFee)
        assertEquals(1, adjustments.refuelCredits.size)
        assertFalse(adjustments.refuel)
        assertTrue(personal.adjustments(oct31, refuel = true).refuel)
        assertEquals(2, personal.adjustments(oct31, false).discounts.size)
    }

    @Test fun `a refuel credit shows the switch only while active`() {
        assertTrue(personal.hasActiveRefuelCredit(oct31))
        assertFalse(personal.toggled("d").hasActiveRefuelCredit(oct31))
    }

    @Test fun `opening a provider uses its once discounts and undo restores them`() {
        val (afterBolt, none) = personal.useOnce("bolt", oct31)
        assertEquals(emptyList<String>(), none)
        assertEquals(personal, afterBolt)
        val (afterMiles, used) = personal.useOnce("miles", oct31)
        assertEquals(listOf("b"), used)
        assertTrue((afterMiles.deals.first { it.id == "b" } as AmountOff).used)
        assertEquals(personal, afterMiles.restore(used))
    }

    @Test fun `deals are added, replaced, toggled and removed by id`() {
        val changed = personal.withDeal(bolt20.copy(basisPoints = 2_500))
        assertEquals(2_500, (changed.deals.first { it.id == "a" } as PercentOff).basisPoints)
        assertEquals(4, changed.deals.size)
        assertEquals(5, personal.withDeal(bolt20.copy(id = "e")).deals.size)
        assertFalse(personal.toggled("a").deals.first { it.id == "a" }.enabled)
        assertEquals(3, personal.withoutDeal("a").deals.size)
    }

    @Test fun `overrides are stored as the core's json and dropped once shared`() {
        val mine = Personal().withOverride("miles.s", listOf(card(89)))
        assertEquals(listOf(card(89)), mine.overrideCards("miles.s"))
        assertEquals(1, mine.adjustments(oct31, false).overrides.size)
        assertEquals(mine, mine.withOverride("miles.s", listOf(card(89))))
        assertEquals(mine, mine.droppingOverridesEqualTo { listOf(card(99)) })
        assertNull(mine.droppingOverridesEqualTo { listOf(card(89)) }.overrideCards("miles.s"))
        assertNull(mine.withoutOverride("miles.s").overrideCards("miles.s"))
    }

    @Test fun `a corrupt override is ignored, not fatal`() {
        val broken = Personal(overrides = listOf(StoredOverride("miles.s", "{")))
        assertNull(broken.overrideCards("miles.s"))
        assertTrue(broken.adjustments(oct31, false).overrides.isEmpty())
    }

    @Test fun `the chosen app is used while the provider still offers it`() {
        val sixt = ProviderInfo("sixt", "SIXT share", "com.trafi.whitelabel.bvg", emptyList(),
            listOf(AppInfo("BVG Jelbi", "com.trafi.whitelabel.bvg"), AppInfo("SIXT", "com.sixt.reservation")))
        assertEquals("com.trafi.whitelabel.bvg", Personal().appFor(sixt))
        assertEquals("com.sixt.reservation", Personal(apps = mapOf("sixt" to "com.sixt.reservation")).appFor(sixt))
        assertEquals("com.trafi.whitelabel.bvg", Personal(apps = mapOf("sixt" to "gone.app")).appFor(sixt))
    }
}
