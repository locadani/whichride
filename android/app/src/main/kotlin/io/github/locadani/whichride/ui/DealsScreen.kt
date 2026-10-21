package io.github.locadani.whichride.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val Space = WhichrideTokens.Space

/** Canvas "Discounts and passes": every deal with an on/off switch; tap a row to edit it. */
@Composable
fun DealsScreen(state: UiState, onBack: () -> Unit, onAdd: () -> Unit, onEdit: (String) -> Unit, onToggle: (String) -> Unit) {
    val colors = WhichrideTheme.colors
    Column(Modifier.fillMaxSize()) {
        TopBar(onBack)
        LazyColumn(
            Modifier.weight(1f),
            contentPadding = PaddingValues(horizontal = Space.md, vertical = Space.xs),
            verticalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            item {
                Column(Modifier.padding(horizontal = Space.xxs), verticalArrangement = Arrangement.spacedBy(Space.xxs)) {
                    Text(stringResource(R.string.deals_title), style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
                    Text(stringResource(R.string.deals_body), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted)
                }
            }
            if (state.personal.deals.isEmpty()) {
                item { Text(stringResource(R.string.deals_empty), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted) }
            }
            items(state.personal.deals, key = { it.id }) { deal ->
                val title = deal.title(state.providers)
                Surface(shape = MaterialTheme.shapes.large, color = colors.surface) {
                    Row(
                        Modifier.clickable { onEdit(deal.id) }.padding(start = Space.md, end = Space.sm, top = Space.sm, bottom = Space.sm),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Space.sm),
                    ) {
                        Monogram(state.providers.nameOf(deal.providerId), large = true)
                        Column(Modifier.weight(1f)) {
                            Text(title, style = MaterialTheme.typography.titleSmall)
                            Text(deal.subtitle(state.today), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
                        }
                        val description = stringResource(R.string.deal_on, title)
                        Switch(
                            checked = deal.enabled,
                            onCheckedChange = { onToggle(deal.id) },
                            modifier = Modifier.semantics { contentDescription = description },
                            colors = SwitchDefaults.colors(
                                checkedTrackColor = colors.accent,
                                checkedThumbColor = colors.onAccent,
                                uncheckedTrackColor = colors.surfaceHighest,
                                uncheckedThumbColor = colors.onSurfaceMuted,
                                uncheckedBorderColor = colors.outline,
                            ),
                        )
                    }
                }
            }
            item { SecondaryButton(stringResource(R.string.deals_add), onAdd, icon = Icons.Filled.Add) }
            item {
                Surface(shape = MaterialTheme.shapes.large, color = colors.background, border = BorderStroke(WhichrideTokens.Size.stroke, colors.outlineVariant)) {
                    Column(Modifier.padding(Space.md), verticalArrangement = Arrangement.spacedBy(Space.xxs)) {
                        Text(stringResource(R.string.deals_refuel_title), style = MaterialTheme.typography.titleSmall)
                        Text(stringResource(R.string.deals_refuel_body), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
                    }
                }
            }
        }
    }
}
