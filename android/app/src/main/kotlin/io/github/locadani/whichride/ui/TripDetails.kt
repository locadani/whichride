package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.IntrinsicSize
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
import androidx.compose.ui.semantics.Role
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
    Row(Modifier.weight(1f).clickable(role = Role.Button, onClick = onToggle), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(
                state.target?.let { stringResource(R.string.trip_to, it.label ?: stringResource(R.string.resolving_unnamed)) }
                    ?: stringResource(R.string.your_trip),
                style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis,
                modifier = Modifier.semantics { heading() },
            )
            // While open, the numbers are in the details; "built-in prices" stays here either way.
            val trip = state.trip?.takeIf { !open }
            val parts = listOfNotNull(
                trip?.let { stringResource(R.string.trip_line, formatKilometres(it.metres, locale()), formatMinutes(it.seconds)) },
                stringResource(R.string.built_in_prices).takeIf { state.priceSource == PriceSource.BUILT_IN },
            )
            if (parts.isNotEmpty()) {
                Text(parts.joinToString(" · "), style = WhichrideTheme.numbers.small, color = colors.onSurfaceMuted)
            }
        }
        Icon(
            if (open) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
            contentDescription = stringResource(if (open) R.string.trip_details_hide else R.string.trip_details_show),
            modifier = Modifier.padding(horizontal = Space.sm),
        )
    }
}

/**
 * The route drawn as on Home (dot, dotted line, square: no "From"/"To" words, which TalkBack still
 * reads), none for a typed trip; then the km and minutes side by side, editable and resettable.
 */
@Composable
fun TripDetails(state: UiState, edit: TripEdit, actions: ResultActions, modifier: Modifier = Modifier) {
    val colors = WhichrideTheme.colors
    Surface(shape = MaterialTheme.shapes.large, color = colors.surface, modifier = modifier) {
        Column(Modifier.padding(Space.md), verticalArrangement = Arrangement.spacedBy(Space.lg)) {
            state.target?.let { target ->
                val to = target.label ?: stringResource(R.string.resolving_unnamed)
                val hasArea = to.contains(", ")
                Row(Modifier.height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(Space.md)) {
                    // Dot on the start's line; square on the middle of the destination's two lines.
                    RouteMarks(Modifier.padding(top = Space.xxs, bottom = if (hasArea) Space.md else Space.xxs))
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                        PlaceLine(
                            stringResource(R.string.trip_from),
                            when (val start = state.start) {
                                Start.MyLocation -> stringResource(R.string.start_my_location)
                                is Start.Chosen -> start.place.label
                            },
                        )
                        PlaceLine(stringResource(R.string.trip_to_label), to)
                    }
                }
            }
            Row(horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                NumberField(
                    stringResource(R.string.trip_distance), edit.km, stringResource(R.string.unit_km),
                    error = stringResource(R.string.manual_km_invalid).takeIf { edit.kmInvalid },
                    reset = if (edit.kmEdited) edit.originalKm else null,
                    resetLabel = { stringResource(R.string.reset_distance, it) },
                    keyboard = KeyboardType.Decimal, onChange = actions.onTripKm, onReset = actions.onResetKm,
                    modifier = Modifier.weight(1f),
                )
                NumberField(
                    stringResource(R.string.trip_time), edit.minutes, stringResource(R.string.unit_min),
                    error = stringResource(R.string.manual_minutes_invalid).takeIf { edit.minutesInvalid },
                    reset = if (edit.minutesEdited) edit.originalMinutes else null,
                    resetLabel = { stringResource(R.string.reset_time, it) },
                    keyboard = KeyboardType.Number, onChange = actions.onTripMinutes, onReset = actions.onResetMinutes,
                    modifier = Modifier.weight(1f),
                )
            }
        }
    }
}

/** "Markthalle Neun / Eisenbahnstraße, Berlin": the name, then the rest of the label; "From: …" for TalkBack. */
@Composable
private fun PlaceLine(key: String, label: String) {
    val colors = WhichrideTheme.colors
    val spoken = "$key: $label"
    Column(Modifier.clearAndSetSemantics { contentDescription = spoken }) {
        Text(label.substringBefore(", "), style = MaterialTheme.typography.titleSmall)
        label.substringAfter(", ", missingDelimiterValue = "").takeIf { it.isNotEmpty() }?.let {
            Text(it, style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
        }
    }
}

@Composable
private fun NumberField(
    label: String,
    value: String,
    unit: String,
    error: String?,
    reset: String?,
    resetLabel: @Composable (String) -> String,
    keyboard: KeyboardType,
    onChange: (String) -> Unit,
    onReset: () -> Unit,
    modifier: Modifier,
) {
    val colors = WhichrideTheme.colors
    Column(modifier) {
        Row(Modifier.heightIn(min = WhichrideTokens.Size.touchTarget), verticalAlignment = Alignment.CenterVertically) {
            Text(label, style = MaterialTheme.typography.labelMedium, color = colors.onSurfaceMuted, modifier = Modifier.weight(1f))
            if (reset != null) {
                val description = resetLabel(reset)
                TextButton(onClick = onReset, modifier = Modifier.semantics { contentDescription = description }) {
                    Icon(Icons.Filled.Refresh, contentDescription = null)
                    Text(reset, color = colors.accentText, style = WhichrideTheme.numbers.small)
                }
            }
        }
        OutlinedTextField(
            value = value,
            onValueChange = onChange,
            modifier = Modifier.fillMaxWidth().semantics { contentDescription = label },
            textStyle = WhichrideTheme.numbers.medium,
            suffix = { Text(unit, style = MaterialTheme.typography.bodySmall) },
            isError = error != null,
            supportingText = error?.let { { Text(it) } },
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
