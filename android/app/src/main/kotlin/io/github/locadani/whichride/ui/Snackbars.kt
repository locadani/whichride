package io.github.locadani.whichride.ui

import androidx.compose.material3.SnackbarDuration
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarResult
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.res.stringResource
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState

/**
 * "MILES −€5.00 used · Undo" after Open <provider> (plan R5). It waits until the user acts on
 * it: they are in the provider's app while a timed snackbar would run out, and Undo is for the
 * trip they decided not to take there (review finding I1).
 */
@Composable
fun OnceUndo(state: UiState, snackbar: SnackbarHostState, onUndo: () -> Unit, onShown: () -> Unit) {
    val usedName = state.usedOnce.firstOrNull()?.let { id -> state.personal.deals.firstOrNull { it.id == id }?.title(state.providers) }
    val usedText = usedName?.let { stringResource(R.string.once_used, it) }
    val undo = stringResource(R.string.undo)
    LaunchedEffect(state.usedOnce) {
        if (usedText == null) return@LaunchedEffect
        val result = snackbar.showSnackbar(usedText, actionLabel = undo, withDismissAction = true, duration = SnackbarDuration.Indefinite)
        if (result == SnackbarResult.ActionPerformed) onUndo() else onShown()
    }
}
