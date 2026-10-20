package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.server.Place

private val Space = WhichrideTokens.Space

/** Change the start (§4 step 3, R20). */
@Composable
fun StartScreen(state: UiState, onBack: () -> Unit, onQuery: (String) -> Unit, onPick: (Place) -> Unit, onMyLocation: () -> Unit) {
    Column(Modifier.fillMaxSize().imePadding()) {
        TopBar(onBack)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md),
            verticalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            Text(stringResource(R.string.start_title), style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
            OutlinedTextField(
                value = state.startQuery,
                onValueChange = onQuery,
                modifier = Modifier.fillMaxWidth(),
                placeholder = { Text(stringResource(R.string.start_hint)) },
                leadingIcon = { Icon(Icons.Filled.Search, contentDescription = null) },
                singleLine = true,
                shape = MaterialTheme.shapes.small,
            )
            Panel {
                PlainRow(Icons.Outlined.LocationOn, stringResource(R.string.use_my_location), null, onMyLocation)
                state.startSuggestions.forEach { place -> PlaceRow(place, Icons.Outlined.Place) { onPick(place) } }
            }
        }
    }
}
