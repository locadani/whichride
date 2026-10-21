package io.github.locadani.whichride.format

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class UnitsTest {
    @Test fun kilometresAcceptADecimalPoint() = assertEquals(7_400u, parseKilometres("7.4"))
    @Test fun kilometresAcceptADecimalComma() = assertEquals(7_400u, parseKilometres(" 7,4 "))
    @Test fun kilometresAcceptWholeNumbersAndMetres() {
        assertEquals(12_000u, parseKilometres("12"))
        assertEquals(12_345u, parseKilometres("12.345"))
        assertEquals(500u, parseKilometres(",5"))
    }
    @Test fun kilometresRefuseNonsenseAndOutOfRange() {
        listOf("", "7.4.1", "7,4,1", "abc", "-3", "0", "0.05", "2000.001", "12.3456", "1e3", "7 4").forEach {
            assertNull(it, parseKilometres(it))
        }
        assertEquals(2_000_000u, parseKilometres("2000"))
        assertEquals(100u, parseKilometres("0.1"))
    }
    @Test fun minutesAreWholeAndBounded() {
        assertEquals(1_320u, parseMinutes(" 22 "))
        listOf("", "0", "1441", "2.5", "-1", "x").forEach { assertNull(it, parseMinutes(it)) }
        assertEquals(86_400u, parseMinutes("1440"))
    }
    @Test fun billedKilometresAreWholeWhenWholeElseOneDecimal() {
        assertEquals("11", formatBilledKilometres(11_000uL, Locale.UK))
        assertEquals("10.2", formatBilledKilometres(10_200uL, Locale.UK))
        assertEquals("10,2", formatBilledKilometres(10_200uL, Locale.ITALY))
        assertEquals("0", formatBilledKilometres(0uL, Locale.UK))
    }
    @Test fun billedMinutesAreWholeWhenWholeElseOneDecimal() {
        assertEquals("25", formatBilledMinutes(1_500uL, Locale.UK))
        assertEquals("22.5", formatBilledMinutes(1_350uL, Locale.UK))
        assertEquals("22,5", formatBilledMinutes(1_350uL, Locale.GERMANY))
    }
    @Test fun kilometresAreShownWithOneDecimalInTheLocalesStyle() {
        assertEquals("7.4", formatKilometres(7_449u, Locale.UK))
        assertEquals("7.5", formatKilometres(7_450u, Locale.UK))
        assertEquals("7,4", formatKilometres(7_400u, Locale.GERMANY))
        assertEquals("0.0", formatKilometres(0u, Locale.UK))
    }
    @Test fun prefilledKilometresKeepEveryMetre() {
        assertEquals("7.4", kilometresInput(7_400u))
        assertEquals("12.345", kilometresInput(12_345u))
        assertEquals("12", kilometresInput(12_000u))
    }
    @Test fun minutesAreRoundedUp() {
        assertEquals("22", formatMinutes(1_320u))
        assertEquals("23", formatMinutes(1_321u))
        assertEquals("0", formatMinutes(0u))
    }
    @Test fun moneyIsEurosInTheLocalesStyle() {
        assertEquals("€9.69", formatMoney(969, Locale.UK))
        assertEquals("9,69 €", formatMoney(969, Locale.GERMANY))
        assertEquals("€0.00", formatMoney(0, Locale.UK))
    }
}
