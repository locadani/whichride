package io.github.locadani.whichride.format

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MoneyInputTest {
    @Test fun `euros accept a decimal comma`() {
        assertEquals(29L, parseEuros("0,29"))
        assertEquals(29L, parseEuros("0.29"))
        assertEquals(150L, parseEuros("1,5"))
        assertEquals(500L, parseEuros("5"))
        assertEquals(500L, parseEuros("€ 5,00"))
        assertEquals(500L, parseEuros("5,00 €"))
        assertEquals(0L, parseEuros("0"))
        assertEquals(29L, parseEuros(",29"))
    }

    @Test fun `euros refuse three decimals and nonsense`() {
        for (text in listOf("", " ", "0,295", "1.2.3", "-1", "abc", "1e3", "12345", "1,", "€")) {
            assertNull(text, parseEuros(text))
        }
    }

    @Test fun `percentages are basis points between 0 and 100`() {
        assertEquals(1_500, parsePercent("15"))
        assertEquals(1_550, parsePercent("15,5"))
        assertEquals(1_525, parsePercent("15.25 %"))
        assertEquals(10_000, parsePercent("100"))
        for (text in listOf("0", "100,01", "101", "", "15,555", "-5")) assertNull(text, parsePercent(text))
    }

    @Test fun `inputs are prefilled with the locale's separator and no needless zeros`() {
        assertEquals("0,29", eurosInput(29, Locale.GERMANY))
        assertEquals("0.29", eurosInput(29, Locale.UK))
        assertEquals("5", eurosInput(500, Locale.ITALY))
        assertEquals("1,50", eurosInput(150, Locale.ITALY))
        assertEquals("15", percentInput(1_500, Locale.GERMANY))
        assertEquals("15,5", percentInput(1_550, Locale.GERMANY))
        assertEquals("15.25", percentInput(1_525, Locale.UK))
    }
}
