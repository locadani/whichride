package io.github.locadani.whichride.personal

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DealFormTest {
    private val id = { "new" }

    @Test fun `a percent discount needs a provider and 0 to 100 percent`() {
        val form = DealForm(providerId = "bolt", kind = DealKind.PERCENT, amount = "20", until = 20_757)
        assertEquals(PercentOff("new", "bolt", 2_000, 20_757) to emptySet<DealField>(), form.validated(id))
        assertEquals(setOf(DealField.PROVIDER), form.copy(providerId = null).validated(id).second)
        assertEquals(setOf(DealField.AMOUNT), form.copy(amount = "120").validated(id).second)
    }

    @Test fun `an amount is euros above zero and may be once`() {
        val form = DealForm(providerId = "miles", kind = DealKind.AMOUNT, amount = "5,00", once = true)
        assertEquals(AmountOff("new", "miles", 500, once = true, until = null), form.validated(id).first)
        assertEquals(setOf(DealField.AMOUNT), form.copy(amount = "0").validated(id).second)
    }

    @Test fun `a pass needs at least one saving`() {
        val form = DealForm(providerId = "miles", kind = DealKind.PASS, amount = "10", waiveUnlock = true)
        assertEquals(Pass("new", "miles", 1_000, 0, 0, waiveUnlock = true), form.validated(id).first)
        val empty = form.copy(amount = "", waiveUnlock = false)
        assertNull(empty.validated(id).first)
        assertEquals(setOf(DealField.AMOUNT), empty.validated(id).second)
        assertEquals(setOf(DealField.TIME_OFF), form.copy(timeOff = "abc").validated(id).second)
    }

    @Test fun `a refuel credit is euros above zero`() {
        val form = DealForm(providerId = "miles", kind = DealKind.REFUEL, amount = "3")
        assertEquals(RefuelCreditDeal("new", "miles", 300), form.validated(id).first)
    }

    @Test fun `editing keeps the id and the enabled state`() {
        val deal = AmountOff("b", "miles", 550, once = true, until = 20_757, enabled = false)
        val form = deal.toForm(Locale.GERMANY)
        assertEquals("5,50", form.amount)
        assertEquals("b", form.editingId)
        assertEquals(deal, form.validated(id).first)
    }
}
