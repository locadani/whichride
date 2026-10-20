package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val FILES = listOf("whichride-LICENSE.txt", "Sora-OFL.txt", "JetBrainsMono-OFL.txt")

/** The licences that travel with the app (R29); copied from the repository at build time. */
@Composable
fun LicencesScreen(onBack: () -> Unit) {
    val assets = LocalContext.current.assets
    val texts = remember { FILES.map { name -> name to assets.open("licences/$name").bufferedReader().use { it.readText() } } }
    Column(Modifier.fillMaxSize()) {
        TopBar(onBack)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = WhichrideTokens.Space.md),
            verticalArrangement = Arrangement.spacedBy(WhichrideTokens.Space.md),
        ) {
            Text(stringResource(R.string.licences), style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
            Text(stringResource(R.string.licences_body), style = MaterialTheme.typography.bodyMedium)
            texts.forEach { (name, text) ->
                Text(name, style = MaterialTheme.typography.titleSmall, modifier = Modifier.semantics { heading() })
                Text(text, style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}
