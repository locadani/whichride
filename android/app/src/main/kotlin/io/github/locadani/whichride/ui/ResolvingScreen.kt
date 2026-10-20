package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.ResolvingStep
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

/** Canvas "Shared from Maps": the place, then what the app is doing. */
@Composable
fun ResolvingScreen(state: UiState, onCancel: () -> Unit) {
    val colors = WhichrideTheme.colors
    Column(Modifier.fillMaxSize()) {
        TopBar(onCancel, close = true)
        Column(
            Modifier.weight(1f).padding(horizontal = WhichrideTokens.Space.xxl),
            verticalArrangement = Arrangement.spacedBy(WhichrideTokens.Space.md, androidx.compose.ui.Alignment.CenterVertically),
        ) {
            Text(stringResource(R.string.resolving_from_maps), style = WhichrideTheme.numbers.small, color = colors.onSurfaceMuted)
            Text(
                state.target?.label ?: stringResource(R.string.resolving_unnamed),
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.semantics { heading() },
            )
            Text(
                stringResource(
                    when (state.step) {
                        ResolvingStep.FINDING_LOCATION -> R.string.resolving_location
                        ResolvingStep.ROUTING -> R.string.resolving_route
                        ResolvingStep.FINDING_PLACE, null -> R.string.resolving_place
                    },
                ),
                style = MaterialTheme.typography.bodyLarge,
                modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite },
            )
            LinearProgressIndicator(Modifier.fillMaxWidth(), color = colors.accent, trackColor = colors.surfaceHighest)
        }
    }
}
