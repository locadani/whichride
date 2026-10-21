package io.github.locadani.whichride.ui

import android.os.Build
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.RadioButtonDefaults
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
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.personal.appFor
import io.github.locadani.whichride.personal.isActive

private val Space = WhichrideTokens.Space

/** Personal settings (R16): discounts, which app opens a provider, language, licences. */
@Composable
fun SettingsScreen(
    state: UiState,
    onBack: () -> Unit,
    onDeals: () -> Unit,
    onChooseApp: (providerId: String, androidPackage: String) -> Unit,
    onLanguage: () -> Unit,
    onLicences: () -> Unit,
) {
    val colors = WhichrideTheme.colors
    Column(Modifier.fillMaxSize()) {
        TopBar(onBack)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md, vertical = Space.xs),
            verticalArrangement = Arrangement.spacedBy(Space.md),
        ) {
            Text(
                stringResource(R.string.settings),
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.padding(horizontal = Space.xxs).semantics { heading() },
            )
            Panel {
                val active = state.personal.deals.count { it.isActive(state.today) }
                NavRow(
                    title = stringResource(R.string.settings_deals),
                    subtitle = pluralStringResource(R.plurals.settings_deals_count, active, active),
                    onClick = onDeals,
                )
            }
            state.providers.filter { it.apps.size > 1 }.forEach { provider ->
                val current = state.personal.appFor(provider)
                Panel {
                    Text(
                        stringResource(R.string.settings_open_with, provider.name),
                        style = MaterialTheme.typography.titleSmall,
                        modifier = Modifier.padding(Space.sm).semantics { heading() },
                    )
                    Column(Modifier.selectableGroup()) {
                        provider.apps.forEach { app ->
                            Row(
                                Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.touchTarget)
                                    .selectable(app.androidPackage == current, role = Role.RadioButton) {
                                        onChooseApp(provider.id, app.androidPackage)
                                    }
                                    .padding(horizontal = Space.sm),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(Space.sm),
                            ) {
                                RadioButton(
                                    selected = app.androidPackage == current,
                                    onClick = null,
                                    colors = RadioButtonDefaults.colors(selectedColor = colors.accent, unselectedColor = colors.outline),
                                )
                                Text(app.name, style = MaterialTheme.typography.bodyLarge)
                            }
                        }
                    }
                }
            }
            Panel {
                Column(Modifier.padding(Space.sm), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
                    Text(stringResource(R.string.settings_language), style = MaterialTheme.typography.titleSmall)
                    Text(stringResource(R.string.settings_language_body), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                        SecondaryButton(stringResource(R.string.settings_language_change), onLanguage)
                    }
                }
            }
            Panel { NavRow(title = stringResource(R.string.licences), subtitle = null, onClick = onLicences) }
        }
    }
}

/** A row that opens another screen. */
@Composable
fun NavRow(title: String, subtitle: String?, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.controlLg)
            .selectable(false, role = Role.Button, onClick = onClick).padding(horizontal = Space.sm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleSmall)
            subtitle?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = WhichrideTheme.colors.onSurfaceMuted) }
        }
        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
    }
}
