package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.personal.DealField
import io.github.locadani.whichride.personal.DealForm
import io.github.locadani.whichride.personal.DealKind
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

private val Space = WhichrideTokens.Space
private const val MILLIS_PER_DAY = 86_400_000L

/** Canvas "New discount": provider, kind, the kind's values, end date or "once". */
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
fun DealEditorScreen(state: UiState, onBack: () -> Unit, onChange: (DealForm) -> Unit, onSave: () -> Unit, onDelete: () -> Unit) {
    val form = state.dealForm ?: return
    val colors = WhichrideTheme.colors
    var picking by rememberSaveable { mutableStateOf(false) }
    Column(Modifier.fillMaxSize().imePadding()) {
        TopBar(onBack, close = true) {
            Text(
                stringResource(if (form.editingId == null) R.string.deal_new else R.string.deal_edit),
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.weight(1f).semantics { heading() },
            )
            if (form.editingId != null) TextButton(onClick = onDelete) { Text(stringResource(R.string.deal_delete), color = colors.error) }
        }
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md, vertical = Space.xs),
            verticalArrangement = Arrangement.spacedBy(Space.lg),
        ) {
            Section(stringResource(R.string.deal_provider)) {
                FlowRow(Modifier.selectableGroup(), horizontalArrangement = Arrangement.spacedBy(Space.xs), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
                    state.providers.forEach { provider ->
                        Choice(provider.name, form.providerId == provider.id) { onChange(form.copy(providerId = provider.id)) }
                    }
                }
                if (DealField.PROVIDER in form.invalid) ErrorText(stringResource(R.string.deal_provider_missing))
            }
            Section(stringResource(R.string.deal_kind)) {
                FlowRow(Modifier.selectableGroup(), horizontalArrangement = Arrangement.spacedBy(Space.xs), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
                    DealKind.entries.forEach { kind ->
                        Choice(stringResource(kind.label()), form.kind == kind) { onChange(form.copy(kind = kind, invalid = emptySet())) }
                    }
                }
            }
            when (form.kind) {
                DealKind.PERCENT -> Amount(R.string.deal_field_percent, form.amount, DealField.AMOUNT in form.invalid, R.string.deal_invalid_percent) { onChange(form.copy(amount = it)) }
                DealKind.AMOUNT -> Amount(R.string.deal_field_amount, form.amount, DealField.AMOUNT in form.invalid, R.string.deal_invalid_amount) { onChange(form.copy(amount = it)) }
                DealKind.REFUEL -> Amount(R.string.deal_field_refuel, form.amount, DealField.AMOUNT in form.invalid, R.string.deal_invalid_amount) { onChange(form.copy(amount = it)) }
                DealKind.PASS -> {
                    Amount(R.string.deal_field_pass_km, form.amount, DealField.AMOUNT in form.invalid, R.string.deal_invalid_pass) { onChange(form.copy(amount = it)) }
                    Amount(R.string.deal_field_pass_minute, form.timeOff, DealField.TIME_OFF in form.invalid, R.string.deal_invalid_percent) { onChange(form.copy(timeOff = it)) }
                    Amount(R.string.deal_field_pass_package, form.packageOff, DealField.PACKAGE_OFF in form.invalid, R.string.deal_invalid_percent) { onChange(form.copy(packageOff = it)) }
                    SwitchRow(stringResource(R.string.deal_field_unlock), form.waiveUnlock) { onChange(form.copy(waiveUnlock = it)) }
                }
            }
            if (form.kind == DealKind.AMOUNT) SwitchRow(stringResource(R.string.deal_field_once), form.once) { onChange(form.copy(once = it)) }
            if (form.kind == DealKind.PERCENT || form.kind == DealKind.AMOUNT) {
                Section(stringResource(R.string.deal_field_until)) {
                    val until = form.until?.let { LocalDate.ofEpochDay(it).format(DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM).withLocale(locale())) }
                    Row(horizontalArrangement = Arrangement.spacedBy(Space.xs), verticalAlignment = Alignment.CenterVertically) {
                        SecondaryButton(until ?: stringResource(R.string.deal_field_until_none), { picking = true }, Modifier.weight(1f))
                        if (form.until != null) TextButton(onClick = { onChange(form.copy(until = null)) }) { Text(stringResource(R.string.deal_field_until_clear)) }
                    }
                }
            }
            Text(stringResource(form.kind.hint()), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
        }
        PrimaryButton(stringResource(R.string.deal_save), onSave, Modifier.padding(Space.md))
    }
    if (picking) {
        val picker = rememberDatePickerState(initialSelectedDateMillis = form.until?.times(MILLIS_PER_DAY))
        DatePickerDialog(
            onDismissRequest = { picking = false },
            confirmButton = {
                TextButton(onClick = {
                    picker.selectedDateMillis?.let { onChange(form.copy(until = it / MILLIS_PER_DAY)) }
                    picking = false
                }) { Text(stringResource(R.string.ok)) }
            },
            dismissButton = { TextButton(onClick = { picking = false }) { Text(stringResource(R.string.cancel)) } },
        ) { DatePicker(picker) }
    }
}

@Composable
private fun Section(title: String, content: @Composable () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(Space.xs)) {
        Text(title, style = MaterialTheme.typography.labelMedium, color = WhichrideTheme.colors.onSurfaceMuted)
        content()
    }
}

@Composable
private fun Choice(label: String, selected: Boolean, onClick: () -> Unit) {
    val colors = WhichrideTheme.colors
    FilterChip(
        selected = selected,
        onClick = onClick,
        label = { Text(label, style = MaterialTheme.typography.labelLarge) },
        modifier = Modifier.heightIn(min = WhichrideTokens.Size.touchTarget),
        shape = MaterialTheme.shapes.extraLarge,
        colors = FilterChipDefaults.filterChipColors(
            selectedContainerColor = colors.accent,
            selectedLabelColor = colors.onAccent,
            labelColor = colors.onSurface,
        ),
        border = FilterChipDefaults.filterChipBorder(enabled = true, selected = selected, borderColor = colors.outline),
    )
}

@Composable
private fun Amount(label: Int, value: String, invalid: Boolean, error: Int, onValue: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onValue,
        label = { Text(stringResource(label)) },
        isError = invalid,
        supportingText = if (invalid) ({ Text(stringResource(error)) }) else null,
        textStyle = WhichrideTheme.numbers.medium,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
}

@Composable
fun SwitchRow(label: String, checked: Boolean, onChecked: (Boolean) -> Unit) {
    val colors = WhichrideTheme.colors
    Row(
        Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.touchTarget)
            .toggleable(checked, role = Role.Switch, onValueChange = onChecked),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
        Switch(
            checked = checked,
            onCheckedChange = null,
            colors = SwitchDefaults.colors(
                checkedTrackColor = colors.accent, checkedThumbColor = colors.onAccent,
                uncheckedTrackColor = colors.surfaceHighest, uncheckedThumbColor = colors.onSurfaceMuted, uncheckedBorderColor = colors.outline,
            ),
        )
    }
}

@Composable
fun ErrorText(text: String) = Text(text, style = MaterialTheme.typography.bodySmall, color = WhichrideTheme.colors.error)

private fun DealKind.label() = when (this) {
    DealKind.PERCENT -> R.string.deal_kind_percent
    DealKind.AMOUNT -> R.string.deal_kind_amount
    DealKind.PASS -> R.string.deal_kind_pass
    DealKind.REFUEL -> R.string.deal_kind_refuel
}

private fun DealKind.hint() = when (this) {
    DealKind.PERCENT -> R.string.deal_hint_percent
    DealKind.AMOUNT -> R.string.deal_hint_amount
    DealKind.PASS -> R.string.deal_hint_pass
    DealKind.REFUEL -> R.string.deal_hint_refuel
}
