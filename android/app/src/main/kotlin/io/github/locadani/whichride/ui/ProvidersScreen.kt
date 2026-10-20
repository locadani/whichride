package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.options.forProvider

private val Space = WhichrideTokens.Space

/** §4.1 step 1: which provider? One large button each, with its offers, lowest price and how many are added. */
@Composable
fun ProvidersScreen(state: UiState, onClose: () -> Unit, onPick: (String) -> Unit) {
    Column(Modifier.fillMaxSize()) {
        TopBar(onClose, close = true) {
            Text(stringResource(R.string.step_1), style = WhichrideTheme.numbers.small, color = WhichrideTheme.colors.onSurfaceMuted)
        }
        Text(
            stringResource(R.string.which_provider),
            style = MaterialTheme.typography.headlineMedium,
            modifier = Modifier.padding(horizontal = Space.lg, vertical = Space.sm).semantics { heading() },
        )
        LazyColumn(contentPadding = PaddingValues(Space.md), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
            items(state.providers, key = { it.id }) { provider ->
                val quotes = state.quotes.filter { it.providerId == provider.id && !it.proposed }
                val chosen = state.choices.forProvider(provider.id, provider.offers.mapTo(mutableSetOf()) { it.id })
                val added = if (chosen.any { it is Choice.AnyOf }) provider.offers.size else chosen.size
                Surface(
                    onClick = { onPick(provider.id) },
                    shape = MaterialTheme.shapes.large,
                    color = WhichrideTheme.colors.surface,
                ) {
                    Row(
                        Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.controlLg).padding(Space.md),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Space.md),
                    ) {
                        Monogram(provider.name, large = true)
                        Column(Modifier.weight(1f)) {
                            Text(provider.name, style = MaterialTheme.typography.titleLarge)
                            quotes.minOfOrNull { it.totalCents }?.let { from ->
                                Text(
                                    pluralStringResource(R.plurals.provider_offers, provider.offers.size, provider.offers.size, money(from)),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = WhichrideTheme.colors.onSurfaceMuted,
                                )
                            }
                        }
                        if (added > 0) Badge(pluralStringResource(R.plurals.added_count, added, added))
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    }
                }
            }
        }
    }
}
