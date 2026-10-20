package io.github.locadani.whichride.designsystem

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color

/**
 * Material 3 colour scheme built only from our roles, so no Material component falls back to
 * Material's default purple. [inversePrimary] is the other theme's accent (snackbar actions on
 * the inverse surface). Light and dark differ only in the values, so one builder serves both.
 */
internal fun WhichrideColors.toMaterial(inversePrimary: Color): ColorScheme = lightColorScheme(
    primary = accent,
    onPrimary = onAccent,
    primaryContainer = surfaceHighest,
    onPrimaryContainer = onSurface,
    inversePrimary = inversePrimary,
    secondary = accent,
    onSecondary = onAccent,
    secondaryContainer = surfaceHighest,
    onSecondaryContainer = onSurface,
    tertiary = accent,
    onTertiary = onAccent,
    tertiaryContainer = surfaceHighest,
    onTertiaryContainer = onSurface,
    background = background,
    onBackground = onSurface,
    surface = background,
    onSurface = onSurface,
    surfaceVariant = surfaceHighest,
    onSurfaceVariant = onSurfaceMuted,
    surfaceTint = background,
    inverseSurface = onSurface,
    inverseOnSurface = background,
    error = error,
    onError = onError,
    errorContainer = surfaceHighest,
    onErrorContainer = onSurface,
    outline = outline,
    outlineVariant = outlineVariant,
    scrim = scrim,
    surfaceBright = surface,
    surfaceContainer = surface,
    surfaceContainerHigh = surfaceHigh,
    surfaceContainerHighest = surfaceHighest,
    surfaceContainerLow = surfaceLow,
    surfaceContainerLowest = background,
    surfaceDim = background,
    primaryFixed = accent,
    primaryFixedDim = accent,
    onPrimaryFixed = onAccent,
    onPrimaryFixedVariant = onAccent,
    secondaryFixed = accent,
    secondaryFixedDim = accent,
    onSecondaryFixed = onAccent,
    onSecondaryFixedVariant = onAccent,
    tertiaryFixed = accent,
    tertiaryFixedDim = accent,
    onTertiaryFixed = onAccent,
    onTertiaryFixedVariant = onAccent,
)
