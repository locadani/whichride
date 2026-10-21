package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Place
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.ManualReason
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val Space = WhichrideTokens.Space

/** Canvas "Route service busy": km and minutes typed from Google Maps (§8). */
@Composable
fun ManualTripScreen(
    state: UiState,
    onBack: () -> Unit,
    onKm: (String) -> Unit,
    onMinutes: (String) -> Unit,
    onSubmit: () -> Unit,
    onOpenMaps: () -> Unit,
    onChooseStart: () -> Unit,
) {
    Column(Modifier.fillMaxSize().imePadding()) {
        TopBar(onBack)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md, vertical = Space.sm),
            verticalArrangement = Arrangement.spacedBy(Space.md),
        ) {
            Text(
                stringResource(
                    when (state.manualReason) {
                        ManualReason.BUSY -> R.string.manual_title_busy
                        ManualReason.NO_ROUTE -> R.string.manual_title_no_route
                        ManualReason.NO_LOCATION -> R.string.manual_title_no_location
                        ManualReason.OFFLINE, null -> R.string.manual_title_offline
                    },
                ),
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.semantics { heading() },
            )
            state.target?.label?.let { Text(stringResource(R.string.trip_to, it), style = MaterialTheme.typography.titleSmall) }
            Text(stringResource(R.string.manual_body), style = MaterialTheme.typography.bodyMedium, color = WhichrideTheme.colors.onSurfaceMuted)
            Row(horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                NumberField(
                    state.manualKm, onKm, stringResource(R.string.manual_km),
                    error = stringResource(R.string.manual_km_invalid).takeIf { state.manualKmInvalid },
                    keyboard = KeyboardType.Decimal, imeAction = ImeAction.Next, onDone = {},
                    modifier = Modifier.weight(1f),
                )
                NumberField(
                    state.manualMinutes, onMinutes, stringResource(R.string.manual_minutes),
                    error = stringResource(R.string.manual_minutes_invalid).takeIf { state.manualMinutesInvalid },
                    keyboard = KeyboardType.Number, imeAction = ImeAction.Done, onDone = onSubmit,
                    modifier = Modifier.weight(1f),
                )
            }
            if (state.manualReason == ManualReason.NO_LOCATION) {
                SecondaryButton(stringResource(R.string.manual_choose_start), onChooseStart, icon = Icons.Outlined.LocationOn)
            }
            if (state.target != null) SecondaryButton(stringResource(R.string.open_in_maps), onOpenMaps, icon = Icons.Filled.Place)
        }
        PrimaryButton(stringResource(R.string.compare), onSubmit, Modifier.padding(Space.md))
    }
}

@Composable
private fun NumberField(
    value: String,
    onChange: (String) -> Unit,
    label: String,
    error: String?,
    keyboard: KeyboardType,
    imeAction: ImeAction,
    onDone: () -> Unit,
    modifier: Modifier,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        modifier = modifier,
        label = { Text(label) },
        isError = error != null,
        supportingText = error?.let { { Text(it) } },
        textStyle = WhichrideTheme.numbers.medium,
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = keyboard, imeAction = imeAction),
        keyboardActions = KeyboardActions(onDone = { onDone() }),
    )
}
