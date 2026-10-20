package io.github.locadani.whichride.format

import java.text.DecimalFormatSymbols
import java.util.Locale

private val DECIMAL = Regex("""(\d{0,4})(?:[.,](\d{1,2}))?""")

/** Whole units and up to two decimals as hundredths, with integer maths; null if not a number. */
private fun hundredths(text: String, unit: Char): Long? {
    val bare = text.trim().removePrefix(unit.toString()).removeSuffix(unit.toString()).trim()
    val match = DECIMAL.matchEntire(bare) ?: return null
    val (whole, fraction) = match.destructured
    if (whole.isEmpty() && fraction.isEmpty()) return null
    return whole.ifEmpty { "0" }.toLong() * 100 + fraction.padEnd(2, '0').ifEmpty { "0" }.toLong()
}

/** Euros typed with a dot or comma ("0,29", "5", "5,00 €") as cents (plan R20). */
fun parseEuros(text: String): Long? = hundredths(text, '€')

/** A percentage above 0 and at most 100, up to two decimals, as basis points. */
fun parsePercent(text: String): Int? = hundredths(text, '%')?.takeIf { it in 1..10_000 }?.toInt()

/** Cents for an input field: "5" for whole euros, else two decimals with the locale's separator. */
fun eurosInput(cents: Long, locale: Locale): String =
    if (cents % 100 == 0L) "${cents / 100}"
    else "${cents / 100}${separator(locale)}${(cents % 100).toString().padStart(2, '0')}"

/** Basis points for an input field: "15", "15,5", "15,25". */
fun percentInput(basisPoints: Int, locale: Locale): String {
    val fraction = (basisPoints % 100).toString().padStart(2, '0').trimEnd('0')
    return if (fraction.isEmpty()) "${basisPoints / 100}" else "${basisPoints / 100}${separator(locale)}$fraction"
}

private fun separator(locale: Locale) = DecimalFormatSymbols.getInstance(locale).decimalSeparator
