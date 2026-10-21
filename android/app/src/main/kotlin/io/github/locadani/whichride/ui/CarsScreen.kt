package io.github.locadani.whichride.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CheckboxDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val Space = WhichrideTokens.Space

/** Offers above this get a search box (§4.1, decision 50). */
private const val SEARCH_ABOVE = 6

/** §4.1 step 2: tick every car of this provider that is nearby. */
@Composable
fun CarsScreen(
    state: UiState,
    providerId: String,
    onBack: () -> Unit,
    onCheckIn: (String) -> Unit,
    onToggle: (Choice) -> Unit,
    onFilter: (String) -> Unit,
    onCommit: () -> Unit,
    onPropose: (String) -> Unit,
) {
    val colors = WhichrideTheme.colors
    val provider = state.providers.firstOrNull { it.id == providerId } ?: return
    val quotes = state.quotes.filter { it.providerId == providerId && !it.proposed }
    val proposals = state.quotes.filter { it.providerId == providerId && it.proposed }
    val anyCar = Choice.AnyOf(providerId)
    val anyOn = anyCar in state.draft
    val terms = state.carFilter.lowercase().split(Regex("\\s+")).filter(String::isNotBlank)
    val matching = quotes.filter { q -> terms.all { q.offerName.lowercase().contains(it) } }.sortedBy { it.offerName }
    Column(Modifier.fillMaxSize().imePadding()) {
        TopBar(onBack) {
            Text(stringResource(R.string.step_2), style = WhichrideTheme.numbers.small, color = colors.onSurfaceMuted, modifier = Modifier.weight(1f))
            OutlinedButton(
                onClick = { onCheckIn(providerId) },
                shape = MaterialTheme.shapes.medium,
                border = BorderStroke(WhichrideTokens.Size.stroke, colors.outline),
            ) {
                Text(stringResource(R.string.check_in, provider.name), color = colors.onSurface)
                Icon(Icons.AutoMirrored.Filled.ArrowForward, contentDescription = null, tint = colors.onSurface)
            }
        }
        Column(Modifier.padding(horizontal = Space.lg, vertical = Space.xs), verticalArrangement = Arrangement.spacedBy(Space.xxs)) {
            Text(
                stringResource(R.string.which_cars, provider.name),
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.semantics { heading() },
            )
            Text(stringResource(R.string.which_cars_body), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted)
        }
        if (provider.offers.size > SEARCH_ABOVE) {
            OutlinedTextField(
                value = state.carFilter,
                onValueChange = onFilter,
                modifier = Modifier.fillMaxWidth().padding(horizontal = Space.md, vertical = Space.xs),
                placeholder = { Text(pluralStringResource(R.plurals.filter_cars, provider.offers.size, provider.offers.size)) },
                leadingIcon = { Icon(Icons.Filled.Search, contentDescription = null) },
                singleLine = true,
                shape = MaterialTheme.shapes.small,
            )
        }
        LazyColumn(
            Modifier.weight(1f),
            contentPadding = PaddingValues(horizontal = Space.md, vertical = Space.xs),
            verticalArrangement = Arrangement.spacedBy(Space.xs),
        ) {
            if (terms.isEmpty()) {
                item {
                    CarRow(
                        label = stringResource(R.string.any_provider_car, provider.name),
                        sub = stringResource(R.string.any_car_sub),
                        price = quotes.minOfOrNull { it.totalCents }?.let { stringResource(R.string.from_price, money(it)) }.orEmpty(),
                        checked = anyOn,
                        strong = true,
                        onToggle = { onToggle(anyCar) },
                    )
                }
            }
            items(matching, key = { it.offerId }) { quote ->
                val choice = Choice.Offer(quote.offerId)
                CarRow(quote.offerName, null, money(quote.totalCents), anyOn || choice in state.draft, strong = false) { onToggle(choice) }
            }
            if (proposals.isNotEmpty() && terms.isEmpty()) {
                item {
                    Text(
                        stringResource(R.string.proposals_title),
                        style = MaterialTheme.typography.labelMedium,
                        color = colors.onSurfaceMuted,
                        modifier = Modifier.padding(top = Space.sm).semantics { heading() },
                    )
                }
                items(proposals, key = { it.offerId }) { quote ->
                    val choice = Choice.Offer(quote.offerId)
                    CarRow(quote.offerName, stringResource(R.string.proposed_meta), money(quote.totalCents), choice in state.draft, strong = false) { onToggle(choice) }
                }
            }
            if (state.crowdEnabled) {
                item {
                    OutlinedButton(
                        onClick = { onPropose(providerId) },
                        modifier = Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.touchTarget).padding(top = Space.xs),
                        shape = MaterialTheme.shapes.medium,
                        border = BorderStroke(WhichrideTokens.Size.stroke, colors.outline),
                    ) { Text(stringResource(R.string.car_not_listed), color = colors.onSurfaceMuted) }
                }
            }
            if (terms.isNotEmpty() && matching.isEmpty()) {
                item {
                    Text(
                        stringResource(R.string.no_car_matches, provider.name, state.carFilter.trim()),
                        style = MaterialTheme.typography.bodyLarge,
                        color = colors.onSurfaceMuted,
                        modifier = Modifier.padding(Space.lg),
                    )
                }
            }
        }
        val label = when {
            state.draft.isEmpty() -> stringResource(R.string.done)
            anyOn -> stringResource(R.string.add_any_car, provider.name)
            else -> pluralStringResource(R.plurals.add_n_cars, state.draft.size, state.draft.size)
        }
        PrimaryButton(label, onCommit, Modifier.padding(Space.md))
    }
}

@Composable
private fun CarRow(label: String, sub: String?, price: String, checked: Boolean, strong: Boolean, onToggle: () -> Unit) {
    val colors = WhichrideTheme.colors
    Surface(
        shape = MaterialTheme.shapes.small,
        color = colors.surface,
        border = if (checked) BorderStroke(WhichrideTokens.Size.strokeStrong, colors.accent) else null,
    ) {
        Row(
            Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.controlLg)
                .toggleable(value = checked, role = Role.Checkbox, onValueChange = { onToggle() })
                .padding(horizontal = Space.sm, vertical = Space.xs),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            Checkbox(
                checked = checked,
                onCheckedChange = null,
                colors = CheckboxDefaults.colors(checkedColor = colors.accent, uncheckedColor = colors.outline, checkmarkColor = colors.onAccent),
            )
            Column(Modifier.weight(1f)) {
                Text(label, style = if (strong) MaterialTheme.typography.titleMedium else MaterialTheme.typography.titleSmall)
                sub?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted) }
            }
            Text(price, style = WhichrideTheme.numbers.medium)
        }
    }
}
