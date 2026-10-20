package io.github.locadani.whichride.format

import java.math.BigDecimal
import java.text.DecimalFormatSymbols
import java.text.NumberFormat
import java.util.Currency
import java.util.Locale

private val KILOMETRES = Regex("""(\d{0,4})(?:[.,](\d{1,3}))?""")
private const val MAX_METRES = 2_000_000u
private const val MIN_METRES = 100u
private const val MAX_MINUTES = 1_440u

/** Kilometres the user typed ("7.4", "7,4", "12") as metres, with integer maths (R24). */
fun parseKilometres(text: String): UInt? {
    val match = KILOMETRES.matchEntire(text.trim()) ?: return null
    val (whole, fraction) = match.destructured
    if (whole.isEmpty() && fraction.isEmpty()) return null
    val metres = (whole.ifEmpty { "0" }.toUInt() * 1_000u) + fraction.padEnd(3, '0').ifEmpty { "0" }.toUInt()
    return metres.takeIf { it in MIN_METRES..MAX_METRES }
}

/** Whole minutes the user typed, as seconds. */
fun parseMinutes(text: String): UInt? =
    text.trim().takeIf { it.all(Char::isDigit) && it.length in 1..4 }?.toUInt()
        ?.takeIf { it in 1u..MAX_MINUTES }?.let { it * 60u }

/** "7.4" (one decimal, half up, the locale's separator); the unit comes from strings.xml. */
fun formatKilometres(metres: UInt, locale: Locale): String {
    val tenths = (metres + 50u) / 100u
    return "${tenths / 10u}${DecimalFormatSymbols.getInstance(locale).decimalSeparator}${tenths % 10u}"
}

/** The value to prefill the km field with: every metre, no trailing zeros, a dot. */
fun kilometresInput(metres: UInt): String {
    val fraction = (metres % 1_000u).toString().padStart(3, '0').trimEnd('0')
    return if (fraction.isEmpty()) "${metres / 1_000u}" else "${metres / 1_000u}.$fraction"
}

/** Started minutes, as providers bill them. */
fun formatMinutes(seconds: UInt): String = ((seconds + 59u) / 60u).toString()

fun formatMoney(cents: Long, locale: Locale): String =
    NumberFormat.getCurrencyInstance(locale).apply { currency = Currency.getInstance("EUR") }
        .format(BigDecimal.valueOf(cents, 2))
