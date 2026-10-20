package io.github.locadani.whichride.ui

import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.locadani.whichride.compare.CompareViewModel

/** Placeholder until the screens exist. */
@Composable
fun WhichrideNav(viewModel: CompareViewModel) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    Text(state.screen.toString())
}
