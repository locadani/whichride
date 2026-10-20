package io.github.locadani.whichride.designsystem

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.LocalMinimumInteractiveComponentSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf

private val LocalWhichrideColors = staticCompositionLocalOf { WhichrideTokens.LightColors }
private val LocalNumberStyles = staticCompositionLocalOf { WhichrideNumberStyles }

/**
 * The app's theme: follows the system's light/dark setting, never the wallpaper (no dynamic
 * colour: decision 51), so every colour pair is one CI has checked for contrast.
 */
@Composable
fun WhichrideTheme(darkTheme: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    val colors = if (darkTheme) WhichrideTokens.DarkColors else WhichrideTokens.LightColors
    val other = if (darkTheme) WhichrideTokens.LightColors else WhichrideTokens.DarkColors
    CompositionLocalProvider(
        LocalWhichrideColors provides colors,
        LocalNumberStyles provides WhichrideNumberStyles,
        LocalMinimumInteractiveComponentSize provides WhichrideTokens.Size.touchTarget,
    ) {
        MaterialTheme(
            colorScheme = colors.toMaterial(inversePrimary = other.accent),
            typography = WhichrideTypography,
            shapes = WhichrideShapes,
            content = content,
        )
    }
}

/** Roles Material has no slot for (hero card, accent text, disabled), and the number styles. */
object WhichrideTheme {
    val colors: WhichrideColors
        @Composable @ReadOnlyComposable get() = LocalWhichrideColors.current
    val numbers: NumberStyles
        @Composable @ReadOnlyComposable get() = LocalNumberStyles.current
}
