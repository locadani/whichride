package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.clickable
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.Start
import io.github.locadani.whichride.compare.TripEdit
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.format.formatKilometres
import io.github.locadani.whichride.format.formatMinutes
import io.github.locadani.whichride.prices.PriceSource

private val Space = WhichrideTokens.Space

/**
 * "To Markthalle Neun ⌄" with the km · min line under it; tapping opens the details, which then
 * hold the numbers instead of that line, so nothing shows twice.
 */
@Composable
fun RowScope.TripTitle(state: UiState, onToggle: () -> Unit) {
    val colors = WhichrideTheme.colors
    val open = state.tripEdit != null
    Row(Modifier.weight(1f).clickable(onClick = onToggle), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(
                state.target?.let { stringResource(R.string.trip_to, it.label ?: stringResource(R.string.resolving_unnamed)) }
                    ?: stringResource(R.string.your_trip),
                style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis,
                modifier = Modifier.semantics { heading() },
            )
            val trip = state.trip
            if (trip != null && !open) {
                val line = stringResource(R.string.trip_line, formatKilometres(trip.metres, locale()), formatMinutes(trip.seconds))
                val builtIn = if (state.priceSource == PriceSource.BUILT_IN) " · " + stringResource(R.string.built_in_prices) else ""
                Text(line + builtIn, style = WhichrideTheme.numbers.small, color = colors.onSurfaceMuted)
            }
        }
        Icon(
            if (open) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
            contentDescription = stringResource(if (open) R.string.trip_details_hide else R.string.trip_details_show),
            modifier = Modifier.padding(horizontal = Space.sm),
        )
    }
}

/** From, to (none for a typed trip), and the km and minutes, editable and resettable. */
@Composable
fun TripDetails(state: UiState, edit: TripEdit, actions: ResultActions, modifier: Modifier = Modifier) {
    val colors = WhichrideTheme.colors
    Surface(shape = MaterialTheme.shapes.large, color = colors.surface, modifier = modifier) {
        Column(Modifier.padding(horizontal = Space.md, vertical = Space.xs)) {
            state.target?.let { target ->
                PlaceLine(
                    stringResource(R.string.trip_from),
                    when (val start = state.start) {
                        Start.MyLocation -> stringResource(R.string.start_my_location)
                        is Start.Chosen -> start.place.label
                    },
                )
                HorizontalDivider(color = colors.outlineVariant)
                PlaceLine(stringResource(R.string.trip_to_label), target.label ?: stringResource(R.string.resolving_unnamed))
                HorizontalDivider(color = colors.outlineVariant)
            }
            NumberLine(
                stringResource(R.string.trip_distance), edit.km, stringResource(R.string.unit_km), edit.kmInvalid,
                reset = if (edit.kmEdited) edit.originalKm else null,
                resetLabel = { stringResource(R.string.reset_distance, it) },
                keyboard = KeyboardType.Decimal, onChange = actions.onTripKm, onReset = actions.onResetKm,
            )
            NumberLine(
                stringResource(R.string.trip_time), edit.minutes, stringResource(R.string.unit_min), edit.minutesInvalid,
                reset = if (edit.minutesEdited) edit.originalMinutes else null,
                resetLabel = { stringResource(R.string.reset_time, it) },
                keyboard = KeyboardType.Number, onChange = actions.onTripMinutes, onReset = actions.onResetMinutes,
            )
        }
    }
}

/** "To  Markthalle Neun / Eisenbahnstraße, Berlin": the name, then the rest of the label. */
@Composable
private fun PlaceLine(key: String, label: String) {
    val colors = WhichrideTheme.colors
    Row(Modifier.padding(vertical = Space.xs), horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
        Text(key, style = MaterialTheme.typography.labelMedium, color = colors.onSurfaceMuted, modifier = Modifier.weight(1f))
        Column(Modifier.weight(4f)) {
            Text(label.substringBefore(", "), style = MaterialTheme.typography.titleSmall)
            label.substringAfter(", ", missingDelimiterValue = "").takeIf { it.isNotEmpty() }?.let {
                Text(it, style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
            }
        }
    }
}

@Composable
private fun NumberLine(
    label: String,
    value: String,
    unit: String,
    invalid: Boolean,
    reset: String?,
    resetLabel: @Composable (String) -> String,
    keyboard: KeyboardType,
    onChange: (String) -> Unit,
    onReset: () -> Unit,
) {
    val colors = WhichrideTheme.colors
    Row(Modifier.padding(vertical = Space.xxs), verticalAlignment = Alignment.CenterVertically) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = colors.onSurfaceMuted, modifier = Modifier.weight(1f))
        if (reset != null) {
            val description = resetLabel(reset)
            TextButton(onClick = onReset, modifier = Modifier.semantics { contentDescription = description }) {
                Icon(Icons.Filled.Refresh, contentDescription = null)
                Text(reset, color = colors.accentText, style = WhichrideTheme.numbers.small)
            }
        }
        OutlinedTextField(
            value = value,
            onValueChange = onChange,
            modifier = Modifier.weight(1f).semantics { contentDescription = label },
            textStyle = WhichrideTheme.numbers.medium,
            suffix = { Text(unit, style = MaterialTheme.typography.bodySmall) },
            isError = invalid,
            singleLine = true,
            keyboardOptions = KeyboardOptions(keyboardType = keyboard, imeAction = ImeAction.Done),
            colors = if (reset != null) {
                OutlinedTextFieldDefaults.colors(focusedBorderColor = colors.accent, unfocusedBorderColor = colors.accent)
            } else {
                OutlinedTextFieldDefaults.colors()
            },
        )
    }
}
