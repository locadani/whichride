package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import io.github.locadani.whichride.R
import io.github.locadani.whichride.core.CardKind
import io.github.locadani.whichride.core.PriceLine
import io.github.locadani.whichride.core.Rounding
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.format.formatBilledKilometres
import io.github.locadani.whichride.format.formatBilledMinutes
import io.github.locadani.whichride.format.percentInput

/** What a price is made of (the core's lines, in the order applied), and how units are counted. */
@Composable
fun PriceLines(lines: List<PriceLine>, muted: Color, modifier: Modifier = Modifier) {
    Column(modifier, verticalArrangement = Arrangement.spacedBy(WhichrideTokens.Space.xxs)) {
        lines.forEach { line ->
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(WhichrideTokens.Space.sm)) {
                Column(Modifier.weight(1f)) {
                    Text(label(line), style = MaterialTheme.typography.bodyMedium)
                    included(line)?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = muted) }
                }
                Text(signed(line.amount()), style = WhichrideTheme.numbers.small)
            }
        }
        startedUnits(lines)?.let { Text(stringResource(it), style = MaterialTheme.typography.bodySmall, color = muted) }
    }
}

@Composable
private fun label(line: PriceLine): String = when (line) {
    is PriceLine.BaseFee -> stringResource(if (line.kind == CardKind.PACKAGE) R.string.line_package else R.string.line_unlock_fee)
    is PriceLine.Distance -> stringResource(R.string.line_distance, formatBilledKilometres(line.billedMetres, locale()), money(line.perKm))
    is PriceLine.Time -> stringResource(R.string.line_time, formatBilledMinutes(line.billedSeconds, locale()), money(line.perMinute))
    is PriceLine.Subscription -> stringResource(R.string.line_subscription)
    is PriceLine.PercentDiscount -> stringResource(R.string.line_percent_discount, percentInput(line.off.toInt(), locale()))
    is PriceLine.AmountDiscount -> stringResource(R.string.line_amount_discount)
    is PriceLine.RefuelCredit -> stringResource(R.string.line_refuel_credit)
    is PriceLine.Minimum -> stringResource(R.string.line_minimum, money(line.minimum))
}

@Composable
private fun included(line: PriceLine): String? = when {
    line is PriceLine.Distance && line.includedKm > 0u ->
        pluralStringResource(R.plurals.line_included_km, line.includedKm.toInt(), line.includedKm.toInt())
    line is PriceLine.Time && line.includedMinutes > 0u ->
        pluralStringResource(R.plurals.line_included_minutes, line.includedMinutes.toInt(), line.includedMinutes.toInt())
    else -> null
}

/** "−€0.73" with a real minus sign; positive amounts as they are. */
@Composable
private fun signed(cents: Long): String = if (cents < 0) "−" + money(-cents) else money(cents)

private fun startedUnits(lines: List<PriceLine>): Int? {
    val km = lines.any { it is PriceLine.Distance && it.rounding == Rounding.STARTED_UNIT && it.amountCents > 0 }
    val minutes = lines.any { it is PriceLine.Time && it.rounding == Rounding.STARTED_UNIT && it.amountCents > 0 }
    return when {
        km && minutes -> R.string.line_started_both
        km -> R.string.line_started_km
        minutes -> R.string.line_started_minutes
        else -> null
    }
}

private fun PriceLine.amount(): Long = when (this) {
    is PriceLine.BaseFee -> amountCents
    is PriceLine.Distance -> amountCents
    is PriceLine.Time -> amountCents
    is PriceLine.Subscription -> amountCents
    is PriceLine.PercentDiscount -> amountCents
    is PriceLine.AmountDiscount -> amountCents
    is PriceLine.RefuelCredit -> amountCents
    is PriceLine.Minimum -> amountCents
}
