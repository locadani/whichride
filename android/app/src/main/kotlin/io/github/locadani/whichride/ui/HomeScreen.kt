package io.github.locadani.whichride.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.draw.clip
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.border
import androidx.compose.foundation.background
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.IconButton
import androidx.compose.material.icons.filled.Settings
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.Start
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.server.Place
import io.github.locadani.whichride.designsystem.R as DesignR

private val Space = WhichrideTokens.Space

/** Canvas "Home: where to?" (spec §4 step 1). */
@Composable
fun HomeScreen(
    state: UiState,
    onQuery: (String) -> Unit,
    onSubmit: () -> Unit,
    onSuggestion: (Place) -> Unit,
    onChangeStart: () -> Unit,
    onSettings: () -> Unit,
) {
    val colors = WhichrideTheme.colors
    Column(Modifier.fillMaxSize().imePadding()) {
        Row(
            Modifier.fillMaxWidth().padding(start = Space.lg, end = Space.sm, top = Space.md),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            Image(painterResource(DesignR.drawable.whichride_logo), contentDescription = null)
            Text(stringResource(R.string.app_name), style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
            IconButton(onClick = onSettings) { Icon(Icons.Filled.Settings, contentDescription = stringResource(R.string.settings)) }
        }
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md, vertical = Space.xl),
            verticalArrangement = Arrangement.spacedBy(Space.md),
        ) {
            Text(
                accented(stringResource(R.string.home_title), stringResource(R.string.home_title_accent)),
                style = MaterialTheme.typography.headlineLarge,
                modifier = Modifier.padding(horizontal = Space.xxs).semantics { heading() },
            )
            Panel {
                // A route from the start (dot) to the destination (square): the start reads as set,
                // the destination as the field still to fill. Drawing only, silent for TalkBack.
                Row(Modifier.height(IntrinsicSize.Min)) {
                    RouteMarks(Modifier.padding(start = Space.sm, top = Space.lg, bottom = Space.lg))
                    Column(Modifier.weight(1f)) {
                        Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(start = Space.sm)) {
                            Text(
                                when (val start = state.start) {
                                    Start.MyLocation -> stringResource(R.string.start_my_location)
                                    is Start.Chosen -> start.place.label
                                },
                                style = MaterialTheme.typography.titleMedium,
                                maxLines = 1,
                                modifier = Modifier.weight(1f),
                            )
                            TextButton(onClick = onChangeStart) { Text(stringResource(R.string.start_change), color = colors.accentText) }
                        }
                        HorizontalDivider(color = colors.outlineVariant)
                        val label = stringResource(R.string.destination_label)
                        TextField(
                            value = state.query,
                            onValueChange = onQuery,
                            modifier = Modifier.fillMaxWidth().semantics { contentDescription = label },
                            placeholder = { Text(stringResource(R.string.destination_hint)) },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Search),
                            keyboardActions = KeyboardActions(onSearch = { onSubmit() }),
                            colors = TextFieldDefaults.colors(
                                focusedContainerColor = colors.surface,
                                unfocusedContainerColor = colors.surface,
                                unfocusedIndicatorColor = colors.surface,
                            ),
                        )
                    }
                }
            }
            state.inputError?.let {
                Text(stringResource(it.message()), color = colors.error, style = MaterialTheme.typography.bodyMedium)
            }
            when {
                state.linkInQuery -> Surface(
                    shape = MaterialTheme.shapes.large,
                    color = colors.background,
                    border = BorderStroke(WhichrideTokens.Size.stroke, colors.accent),
                ) {
                    Column(Modifier.padding(Space.md), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                        Text(stringResource(R.string.maps_link_found), style = MaterialTheme.typography.labelMedium, color = colors.accentText)
                        PrimaryButton(stringResource(R.string.compare_this_trip), onSubmit)
                    }
                }
                state.suggestions.isNotEmpty() -> Panel {
                    state.suggestions.forEach { place -> PlaceRow(place, Icons.Outlined.Place) { onSuggestion(place) } }
                }
                state.suggestionsUnavailable -> Text(
                    stringResource(R.string.suggestions_unavailable),
                    style = MaterialTheme.typography.bodySmall,
                    color = colors.onSurfaceMuted,
                )
                else -> Text(
                    stringResource(R.string.home_tip),
                    style = MaterialTheme.typography.bodySmall,
                    color = colors.onSurfaceMuted,
                    modifier = Modifier.padding(horizontal = Space.xxs),
                )
            }
        }
    }
}

/** Filled dot, dotted line, hollow square: start, route, destination. */
@Composable
private fun RouteMarks(modifier: Modifier) {
    val colors = WhichrideTheme.colors
    val stroke = WhichrideTokens.Size.strokeStrong
    Column(modifier.fillMaxHeight(), horizontalAlignment = Alignment.CenterHorizontally) {
        Box(Modifier.size(Space.sm).clip(CircleShape).background(colors.accent))
        Canvas(Modifier.weight(1f).width(stroke).padding(vertical = Space.xxs)) {
            val dash = Space.xxs.toPx()
            drawLine(
                colors.outline,
                Offset(size.width / 2, 0f),
                Offset(size.width / 2, size.height),
                strokeWidth = stroke.toPx(),
                pathEffect = PathEffect.dashPathEffect(floatArrayOf(dash, dash)),
            )
        }
        Box(Modifier.size(Space.sm).border(stroke, colors.onSurfaceMuted, RoundedCornerShape(Space.xxs)))
    }
}

