package io.github.locadani.whichride.designsystem

import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color

/** One theme's colour roles, named as in design/tokens.json (`theme.<mode>.<role>`). */
@Immutable
data class WhichrideColors(
    val accent: Color,
    val accentText: Color,
    val background: Color,
    val disabled: Color,
    val error: Color,
    val hero: Color,
    val onAccent: Color,
    val onDisabled: Color,
    val onError: Color,
    val onHero: Color,
    val onSurface: Color,
    val onSurfaceMuted: Color,
    val outline: Color,
    val outlineVariant: Color,
    val scrim: Color,
    val surface: Color,
    val surfaceHigh: Color,
    val surfaceHighest: Color,
    val surfaceLow: Color,
)
