package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.reportLimits
import io.github.locadani.whichride.crowd.PriceField
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.crowd.PricePurpose
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val Space = WhichrideTokens.Space

/** "Price wrong? Tell us" (§7.1) and "Car not listed? Add it" (§5.1): one form, two purposes. */
@Composable
fun PriceFormScreen(state: UiState, onBack: () -> Unit, onChange: (PriceForm) -> Unit, onSave: () -> Unit, onReset: (String) -> Unit) {
    val form = state.priceForm ?: return
    val colors = WhichrideTheme.colors
    val purpose = form.purpose
    val provider = state.providers.nameOf(
        when (purpose) {
            is PricePurpose.Report -> purpose.providerId
            is PricePurpose.Propose -> purpose.providerId
        },
    )
    Column(Modifier.fillMaxSize().imePadding()) {
        TopBar(onBack, close = true)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md),
            verticalArrangement = Arrangement.spacedBy(Space.md),
        ) {
            when (purpose) {
                is PricePurpose.Report -> {
                    Title(stringResource(R.string.price_title, provider))
                    Text(stringResource(R.string.price_body, purpose.offerName, provider), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted)
                }
                is PricePurpose.Propose -> {
                    Title(stringResource(R.string.propose_title, provider))
                    Text(stringResource(R.string.propose_body, provider), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted)
                }
            }
            PriceFields(form, onChange)
            Text(
                stringResource(if (purpose is PricePurpose.Propose) R.string.propose_note else R.string.price_note),
                style = MaterialTheme.typography.bodySmall,
                color = colors.onSurfaceMuted,
            )
            if (purpose is PricePurpose.Report && state.quotes.any { it.offerId == purpose.offerId && it.overridden }) {
                TextButton(onClick = { onReset(purpose.offerId) }) { Text(stringResource(R.string.price_reset), color = colors.accentText) }
            }
        }
        PrimaryButton(
            stringResource(if (purpose is PricePurpose.Propose) R.string.propose_save else R.string.price_save),
            onSave,
            Modifier.padding(Space.md),
            enabled = !form.sending,
        )
    }
}

@Composable
private fun Title(text: String) =
    Text(text, style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })

/** The name (proposals only) and the four prices, each marked when out of range. */
@Composable
fun PriceFields(form: PriceForm, onChange: (PriceForm) -> Unit) {
    val limits = reportLimits()
    Column(verticalArrangement = Arrangement.spacedBy(Space.xs)) {
        if (form.purpose is PricePurpose.Propose) {
            OutlinedTextField(
                value = form.name,
                onValueChange = { onChange(form.copy(name = it)) },
                label = { Text(stringResource(R.string.field_name)) },
                isError = PriceField.NAME in form.invalid,
                supportingText = if (PriceField.NAME in form.invalid) ({ Text(stringResource(R.string.field_invalid_name, limits.maxNameChars.toInt())) }) else null,
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words),
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
        }
        Euros(R.string.field_unlock, form.unlock, PriceField.UNLOCK in form.invalid, limits.maxBaseFee) { onChange(form.copy(unlock = it)) }
        Euros(R.string.field_per_km, form.perKm, PriceField.PER_KM in form.invalid, limits.maxPerUnit) { onChange(form.copy(perKm = it)) }
        Euros(R.string.field_per_minute, form.perMinute, PriceField.PER_MINUTE in form.invalid, limits.maxPerUnit) { onChange(form.copy(perMinute = it)) }
        Euros(R.string.field_minimum, form.minimum, PriceField.MINIMUM in form.invalid, limits.maxMinimum) { onChange(form.copy(minimum = it)) }
    }
}

@Composable
private fun Euros(label: Int, value: String, invalid: Boolean, max: Long, onValue: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onValue,
        label = { Text(stringResource(label)) },
        isError = invalid,
        supportingText = if (invalid) ({ Text(stringResource(R.string.field_invalid_euros, money(max))) }) else null,
        textStyle = WhichrideTheme.numbers.medium,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
}
