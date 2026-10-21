package io.github.locadani.whichride.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.QuestionStep
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.crowd.editable
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val Space = WhichrideTokens.Space

data class QuestionActions(
    val onYes: () -> Unit = {},
    val onNo: () -> Unit = {},
    val onDontKnow: () -> Unit = {},
    /** Skip, Close, or a swipe down. */
    val onClose: () -> Unit = {},
    val onChange: (PriceForm) -> Unit = {},
    val onSave: () -> Unit = {},
)

/** Canvas "Price check" (§7.2) as a bottom sheet over the result. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QuestionSheet(state: UiState, actions: QuestionActions) {
    if (state.question == null) return
    ModalBottomSheet(
        onDismissRequest = actions.onClose,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        containerColor = WhichrideTheme.colors.surfaceLow,
        contentColor = WhichrideTheme.colors.onSurface,
        scrimColor = WhichrideTheme.colors.scrim,
    ) { QuestionContent(state, actions) }
}

@Composable
fun QuestionContent(state: UiState, actions: QuestionActions) {
    val asked = state.question ?: return
    val colors = WhichrideTheme.colors
    val provider = state.providers.nameOf(asked.providerId)
    Column(
        Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(start = Space.lg, end = Space.lg, bottom = Space.xl),
        verticalArrangement = Arrangement.spacedBy(Space.md),
    ) {
        when (asked.step) {
            QuestionStep.ASK -> {
                Text(stringResource(R.string.question_back_from, provider), style = WhichrideTheme.numbers.small, color = colors.onSurfaceMuted)
                Text(
                    stringResource(if (asked.isProposal) R.string.question_title_proposal else R.string.question_title, provider, asked.offerName),
                    style = MaterialTheme.typography.headlineSmall,
                    modifier = Modifier.semantics { heading() },
                )
                asked.question.cards.editable()?.let { card ->
                    Row(horizontalArrangement = Arrangement.spacedBy(Space.xs)) {
                        Value(stringResource(R.string.question_per_km), money(card.perKm), Modifier.weight(1f))
                        Value(stringResource(R.string.question_per_minute), money(card.perMinute), Modifier.weight(1f))
                        Value(stringResource(R.string.question_unlock), money(card.baseFee), Modifier.weight(1f))
                    }
                }
                PrimaryButton(stringResource(R.string.question_yes), actions.onYes)
                Row(horizontalArrangement = Arrangement.spacedBy(Space.xs)) {
                    SecondaryButton(stringResource(if (asked.isProposal) R.string.question_no else R.string.question_no_fix), actions.onNo, Modifier.weight(1f))
                    TextButton(onClick = actions.onClose, modifier = Modifier.weight(1f)) {
                        Text(stringResource(R.string.question_skip), color = colors.onSurfaceMuted, style = MaterialTheme.typography.labelLarge)
                    }
                }
                Text(stringResource(R.string.question_note), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
            }
            QuestionStep.FIX -> {
                Text(stringResource(R.string.price_title, provider), style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })
                Text(stringResource(R.string.price_body, asked.offerName, provider), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted)
                state.priceForm?.let { PriceFields(it, actions.onChange) }
                Text(stringResource(R.string.price_note), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
                PrimaryButton(stringResource(R.string.price_save), actions.onSave)
                TextButton(onClick = actions.onDontKnow, modifier = Modifier.fillMaxWidth()) {
                    Text(stringResource(R.string.question_dont_know), color = colors.onSurfaceMuted)
                }
            }
            QuestionStep.CONFIRMED, QuestionStep.CORRECTED, QuestionStep.NOT_SENT -> {
                Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                    if (asked.step != QuestionStep.NOT_SENT) {
                        Box(Modifier.size(WhichrideTokens.Size.controlLg).clip(CircleShape).background(colors.accent), contentAlignment = Alignment.Center) {
                            Icon(Icons.Filled.Check, contentDescription = null, tint = colors.onAccent)
                        }
                        Text(stringResource(R.string.question_thanks), style = MaterialTheme.typography.titleLarge, modifier = Modifier.semantics { heading() })
                    }
                    Text(
                        stringResource(
                            when (asked.step) {
                                QuestionStep.CONFIRMED -> R.string.question_confirmed
                                QuestionStep.CORRECTED -> R.string.question_corrected
                                else -> R.string.question_not_sent
                            },
                        ),
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.onSurfaceMuted,
                        textAlign = TextAlign.Center,
                    )
                }
                SecondaryButton(stringResource(R.string.close), actions.onClose)
            }
        }
    }
}

@Composable
private fun Value(label: String, value: String, modifier: Modifier) {
    Surface(modifier, shape = MaterialTheme.shapes.large, color = WhichrideTheme.colors.surface) {
        Column(Modifier.padding(Space.sm), verticalArrangement = Arrangement.spacedBy(Space.xxs)) {
            Text(label, style = MaterialTheme.typography.labelSmall, color = WhichrideTheme.colors.onSurfaceMuted)
            Text(value, style = WhichrideTheme.numbers.medium)
        }
    }
}
