// Generated from design/tokens.json by design/build.mjs. Do not edit.
package io.github.locadani.whichride.designsystem

import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** Values from design/tokens.json. Use [WhichrideTheme] for colours and text styles. */
object WhichrideTokens {
    val LightColors: WhichrideColors = WhichrideColors(
        accent = Color(0xFF0B7A50),
        accentText = Color(0xFF0B7A50),
        background = Color(0xFFF3F6F8),
        disabled = Color(0xFFE3E9EE),
        error = Color(0xFFB3261E),
        hero = Color(0xFF0B1218),
        onAccent = Color(0xFFFFFFFF),
        onDisabled = Color(0xFF4D5966),
        onError = Color(0xFFFFFFFF),
        onHero = Color(0xFFE8EEF5),
        onSurface = Color(0xFF0B1218),
        onSurfaceMuted = Color(0xFF4D5966),
        outline = Color(0xFF4D5966),
        outlineVariant = Color(0xFFCBD4DC),
        scrim = Color(0xFFC9D1D8),
        surface = Color(0xFFFFFFFF),
        surfaceHigh = Color(0xFFEEF2F5),
        surfaceHighest = Color(0xFFE3E9EE),
        surfaceLow = Color(0xFFFFFFFF),
    )

    val DarkColors: WhichrideColors = WhichrideColors(
        accent = Color(0xFF3DDC97),
        accentText = Color(0xFF3DDC97),
        background = Color(0xFF0B0F14),
        disabled = Color(0xFF1D252F),
        error = Color(0xFFF2B8B5),
        hero = Color(0xFF3DDC97),
        onAccent = Color(0xFF04261A),
        onDisabled = Color(0xFF9AA7B6),
        onError = Color(0xFF601410),
        onHero = Color(0xFF04261A),
        onSurface = Color(0xFFE8EEF5),
        onSurfaceMuted = Color(0xFF9AA7B6),
        outline = Color(0xFF9AA7B6),
        outlineVariant = Color(0xFF2C3642),
        scrim = Color(0xFF06090C),
        surface = Color(0xFF151B23),
        surfaceHigh = Color(0xFF1D252F),
        surfaceHighest = Color(0xFF232B36),
        surfaceLow = Color(0xFF11161D),
    )

    val BrandMark: Color = Color(0xFF3DDC97)
    val BrandOnMark: Color = Color(0xFF04261A)

    object Space {
        val lg: Dp = 20.dp
        val md: Dp = 16.dp
        val sm: Dp = 12.dp
        val xl: Dp = 24.dp
        val xs: Dp = 8.dp
        val xxl: Dp = 32.dp
        val xxs: Dp = 4.dp
        val xxxl: Dp = 40.dp
    }

    object Radius {
        val full: Dp = 999.dp
        val lg: Dp = 16.dp
        val md: Dp = 14.dp
        val sm: Dp = 10.dp
        val xl: Dp = 20.dp
        val xxl: Dp = 28.dp
    }

    object Size {
        val controlLg: Dp = 56.dp
        val controlMd: Dp = 48.dp
        val monogram: Dp = 32.dp
        val stroke: Dp = 1.dp
        val strokeStrong: Dp = 2.dp
        val touchTarget: Dp = 48.dp
    }

    internal object Type {
        val weights: List<Int> = listOf(400, 500, 600, 700)
        val bodyLarge: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 16f, weight = 400, lineHeight = 1.5f, letterSpacingEm = 0f)
        val bodyMedium: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 14f, weight = 400, lineHeight = 1.55f, letterSpacingEm = 0f)
        val bodySmall: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 13f, weight = 400, lineHeight = 1.5f, letterSpacingEm = 0f)
        val headlineLarge: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 30f, weight = 700, lineHeight = 1.1f, letterSpacingEm = -0.03f)
        val headlineMedium: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 26f, weight = 700, lineHeight = 1.15f, letterSpacingEm = -0.02f)
        val headlineSmall: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 22f, weight = 700, lineHeight = 1.25f, letterSpacingEm = -0.02f)
        val labelLarge: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 16f, weight = 700, lineHeight = 1.25f, letterSpacingEm = 0f)
        val labelMedium: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 13f, weight = 600, lineHeight = 1.3f, letterSpacingEm = 0f)
        val labelSmall: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 12f, weight = 600, lineHeight = 1.3f, letterSpacingEm = 0.06f)
        val numberLarge: TypeToken = TypeToken(FontRole.NUMBER, sizeSp = 36f, weight = 700, lineHeight = 1f, letterSpacingEm = -0.02f)
        val numberMedium: TypeToken = TypeToken(FontRole.NUMBER, sizeSp = 18f, weight = 700, lineHeight = 1.3f, letterSpacingEm = 0f)
        val numberSmall: TypeToken = TypeToken(FontRole.NUMBER, sizeSp = 12f, weight = 500, lineHeight = 1.4f, letterSpacingEm = 0f)
        val titleLarge: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 20f, weight = 700, lineHeight = 1.2f, letterSpacingEm = 0f)
        val titleMedium: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 16f, weight = 600, lineHeight = 1.35f, letterSpacingEm = 0f)
        val titleSmall: TypeToken = TypeToken(FontRole.TEXT, sizeSp = 15f, weight = 600, lineHeight = 1.35f, letterSpacingEm = 0f)
    }
}
