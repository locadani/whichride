package io.github.locadani.whichride.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import io.github.locadani.whichride.personal.hasActiveRefuelCredit
import io.github.locadani.whichride.personal.isActive
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.KeyboardArrowUp
import androidx.compose.material.icons.filled.Place
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedIconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import io.github.locadani.whichride.compare.canReport
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.format.formatKilometres
import io.github.locadani.whichride.format.formatMinutes
import io.github.locadani.whichride.prices.PriceSource

private val Space = WhichrideTokens.Space
private val Size = WhichrideTokens.Size

/** Everything the result screen can ask the ViewModel to do. */
data class ResultActions(
    val onBack: () -> Unit = {},
    val onEdit: () -> Unit = {},
    val onChangeStart: () -> Unit = {},
    val onAddCars: () -> Unit = {},
    val onRemove: (Choice) -> Unit = {},
    val onSelect: (Choice) -> Unit = {},
    val onOpen: () -> Unit = {},
    val onOpenMaps: () -> Unit = {},
    val onDeals: () -> Unit = {},
    val onRefuel: (Boolean) -> Unit = {},
    val onReport: () -> Unit = {},
)

/** Canvas "Result + add the cars near you" (spec §4 steps 6–7, §4.1). */
@Composable
fun ResultScreen(state: UiState, actions: ResultActions) {
    val colors = WhichrideTheme.colors
    val ranked = state.options?.ranked.orEmpty()
    val chosen = ranked.firstOrNull { it.choice == state.selected } ?: ranked.firstOrNull()
    // Closed each time the screen opens; each option on its own.
    var open by rememberSaveable { mutableStateOf(listOf<String>()) }
    val details = { option: RankedOption ->
        Details(option.choice.toString() in open) {
            val key = option.choice.toString()
            open = if (key in open) open - key else open + key
        }
    }
    Column(Modifier.fillMaxSize()) {
        TopBar(actions.onBack) {
            Column(Modifier.weight(1f)) {
                Text(
                    stringResource(R.string.trip_to, state.target?.label ?: stringResource(R.string.resolving_unnamed)),
                    style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.semantics { heading() },
                )
                state.trip?.let { trip ->
                    val line = stringResource(R.string.trip_line, formatKilometres(trip.metres, locale()), formatMinutes(trip.seconds))
                    val builtIn = if (state.priceSource == PriceSource.BUILT_IN) " · " + stringResource(R.string.built_in_prices) else ""
                    Text(line + builtIn, style = WhichrideTheme.numbers.small, color = colors.onSurfaceMuted)
                }
            }
        }
        // Own row, wrapping: beside the title, long labels (Italian, 200 % font) crushed the trip line.
        FlowRow(
            Modifier.fillMaxWidth().padding(horizontal = Space.md),
            horizontalArrangement = Arrangement.spacedBy(Space.xxs),
            itemVerticalAlignment = Alignment.CenterVertically,
        ) {
            val active = state.personal.deals.count { it.isActive(state.today) }
            Surface(
                onClick = actions.onDeals,
                shape = RoundedCornerShape(WhichrideTokens.Radius.full),
                color = colors.surface,
                modifier = Modifier.heightIn(min = Size.touchTarget),
            ) {
                Text(
                    if (active == 0) stringResource(R.string.discounts_pill_none)
                    else pluralStringResource(R.plurals.discounts_pill, active, active),
                    style = MaterialTheme.typography.labelMedium,
                    modifier = Modifier.wrapContentHeight().padding(horizontal = Space.sm, vertical = Space.xs),
                )
            }
            TextButton(onClick = actions.onChangeStart) { Text(stringResource(R.string.change_start), color = colors.accentText) }
            TextButton(onClick = actions.onEdit) { Text(stringResource(R.string.edit), color = colors.accentText) }
    }
        val options = state.options
        if (options == null) {
            Column(Modifier.weight(1f).padding(Space.md), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                Text(stringResource(R.string.loading_prices), style = MaterialTheme.typography.bodyMedium)
                LinearProgressIndicator(Modifier.fillMaxWidth(), color = colors.accent, trackColor = colors.surfaceHighest)
            }
        } else {
            LazyColumn(
                Modifier.weight(1f),
                contentPadding = PaddingValues(horizontal = Space.md, vertical = Space.sm),
                verticalArrangement = Arrangement.spacedBy(Space.xs),
            ) {
                if (state.personal.hasActiveRefuelCredit(state.today)) {
                    item { SwitchRow(stringResource(R.string.refuel_switch), state.refuel, actions.onRefuel) }
                }
                if (ranked.isEmpty()) {
                    item { Intro(state) }
                } else {
                    item {
                        Hero(ranked.first(), state, selected = chosen == ranked.first() && ranked.size > 1, details(ranked.first()), actions.onSelect, actions.onRemove)
                    }
                    items(ranked.drop(1), key = { it.choice.toString() }) { option ->
                        OptionRow(option, ranked.first().quote.totalCents, state, selected = chosen == option, details(option), actions.onSelect, actions.onRemove)
                    }
                }
                item {
                    if (ranked.isEmpty()) PrimaryButton(stringResource(R.string.add_cars), actions.onAddCars, icon = Icons.Filled.Add)
                    else SecondaryButton(stringResource(R.string.add_more_cars), actions.onAddCars, icon = Icons.Filled.Add)
                }
                if (ranked.isNotEmpty()) item { CheaperLine(state) }
                if (state.canReport()) {
                    item {
                        TextButton(onClick = actions.onReport) {
                            Text(stringResource(R.string.report_price), color = WhichrideTheme.colors.accentText)
                        }
                    }
                }
            }
        }
        Row(
            Modifier.padding(horizontal = Space.md, vertical = Space.sm),
            horizontalArrangement = Arrangement.spacedBy(Space.sm),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedIconButton(
                onClick = actions.onOpenMaps,
                modifier = Modifier.size(Size.controlLg),
                shape = MaterialTheme.shapes.medium,
                border = BorderStroke(Size.stroke, colors.outline),
                colors = IconButtonDefaults.outlinedIconButtonColors(containerColor = colors.surface),
            ) { Icon(Icons.Filled.Place, contentDescription = stringResource(R.string.open_in_maps)) }
            PrimaryButton(
                text = chosen?.let { stringResource(R.string.open_provider, state.providers.nameOf(it.quote.providerId)) }
                    ?: stringResource(R.string.add_cars_to_compare),
                onClick = actions.onOpen,
                enabled = chosen != null,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun Intro(state: UiState) {
    Panel {
        Column(Modifier.padding(Space.md), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
            Text(
                accented(stringResource(R.string.intro_title), stringResource(R.string.intro_title_accent)),
                style = MaterialTheme.typography.headlineSmall,
            )
            Text(
                stringResource(R.string.intro_body, orList(state.providers.map { it.name })),
                style = MaterialTheme.typography.bodyMedium,
                color = WhichrideTheme.colors.onSurfaceMuted,
            )
        }
    }
}

@Composable
private fun Hero(option: RankedOption, state: UiState, selected: Boolean, details: Details, onSelect: (Choice) -> Unit, onRemove: (Choice) -> Unit) {
    val colors = WhichrideTheme.colors
    val provider = state.providers.nameOf(option.quote.providerId)
    Surface(
        shape = MaterialTheme.shapes.large,
        color = colors.hero,
        contentColor = colors.onHero,
        border = if (selected) BorderStroke(Size.strokeStrong, colors.onSurface) else null,
    ) {
        Column {
            Row(verticalAlignment = Alignment.Bottom) {
                Column(
                    Modifier.weight(1f).selectable(selected, onClick = { onSelect(option.choice) }, role = Role.RadioButton)
                        .padding(start = Space.lg, top = Space.md, bottom = Space.md),
                    verticalArrangement = Arrangement.spacedBy(Space.xs),
                ) {
                    Text(stringResource(R.string.cheapest_of_options).uppercase(locale()), style = MaterialTheme.typography.labelSmall)
                    Row(verticalAlignment = Alignment.Bottom) {
                        Column(Modifier.weight(1f)) {
                            Text(provider, style = MaterialTheme.typography.titleLarge)
                            Text("${offerLabel(option)} · ${optionMeta(option, state.priceSource)}", style = MaterialTheme.typography.bodySmall)
                        }
                        Text(money(option.quote.totalCents), style = WhichrideTheme.numbers.large)
                    }
                }
                DetailsButton(option, provider, details)
                RemoveButton(option, provider, onRemove)
            }
            if (details.open) {
                PriceLines(option.quote.lines, muted = colors.onHero, Modifier.padding(start = Space.lg, end = Space.md, bottom = Space.md))
            }
        }
    }
}

@Composable
private fun OptionRow(
    option: RankedOption,
    best: Long,
    state: UiState,
    selected: Boolean,
    details: Details,
    onSelect: (Choice) -> Unit,
    onRemove: (Choice) -> Unit,
) {
    val colors = WhichrideTheme.colors
    val provider = state.providers.nameOf(option.quote.providerId)
    Surface(
        shape = MaterialTheme.shapes.small,
        color = colors.surface,
        border = if (selected) BorderStroke(Size.strokeStrong, colors.accent) else null,
    ) {
        Column {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Row(
                    Modifier.weight(1f).heightIn(min = Size.controlLg)
                        .selectable(selected, onClick = { onSelect(option.choice) }, role = Role.RadioButton)
                        .padding(start = Space.md, top = Space.xs, bottom = Space.xs),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(Space.sm),
                ) {
                    Monogram(provider)
                    Column(Modifier.weight(1f)) {
                        Text("$provider ${offerLabel(option)}", style = MaterialTheme.typography.titleSmall)
                        Text(optionMeta(option, state.priceSource), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
                    }
                    Column(horizontalAlignment = Alignment.End) {
                        Text(money(option.quote.totalCents), style = WhichrideTheme.numbers.medium)
                        Text(
                            stringResource(R.string.price_difference, money(option.quote.totalCents - best)),
                            style = WhichrideTheme.numbers.small,
                            color = colors.onSurfaceMuted,
                        )
                    }
                }
                DetailsButton(option, provider, details)
                RemoveButton(option, provider, onRemove)
            }
            if (details.open) {
                PriceLines(
                    option.quote.lines,
                    muted = colors.onSurfaceMuted,
                    Modifier.padding(start = Space.md + Size.monogram + Space.sm, end = Space.md, bottom = Space.sm),
                )
            }
        }
    }
}

/** Whether an option's price details are showing, and how to show or hide them. */
data class Details(val open: Boolean, val toggle: () -> Unit)

@Composable
private fun DetailsButton(option: RankedOption, provider: String, details: Details) {
    IconButton(onClick = details.toggle) {
        Icon(
            if (details.open) Icons.Filled.KeyboardArrowUp else Icons.Filled.KeyboardArrowDown,
            contentDescription = stringResource(
                if (details.open) R.string.price_details_hide else R.string.price_details_show, provider, offerLabel(option),
            ),
        )
    }
}

@Composable
private fun RemoveButton(option: RankedOption, provider: String, onRemove: (Choice) -> Unit) {
    IconButton(onClick = { onRemove(option.choice) }) {
        Icon(Icons.Filled.Close, contentDescription = stringResource(R.string.remove_option, provider, offerLabel(option)))
    }
}

@Composable
private fun CheaperLine(state: UiState) {
    val cheaper = state.options?.cheaperNotAdded.orEmpty()
    val text = cheaper.firstOrNull()?.let { first ->
        pluralStringResource(
            R.plurals.cheaper_not_added, cheaper.size, cheaper.size, money(first.totalCents),
            "${state.providers.nameOf(first.providerId)} ${first.offerName}",
        )
    } ?: stringResource(R.string.nothing_cheaper)
    Text(text, style = MaterialTheme.typography.bodySmall, color = WhichrideTheme.colors.onSurfaceMuted, modifier = Modifier.padding(Space.xxs))
}
