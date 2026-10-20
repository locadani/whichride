package io.github.locadani.whichride.designsystem

import androidx.compose.material3.Typography
import androidx.compose.runtime.Immutable
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp

private fun variableFamily(resId: Int) = FontFamily(
    WhichrideTokens.Type.weights.map { w ->
        Font(resId, FontWeight(w), variationSettings = FontVariation.Settings(FontVariation.weight(w)))
    },
)

internal val Sora = variableFamily(R.font.sora)
internal val JetBrainsMono = variableFamily(R.font.jetbrains_mono)

internal fun TypeToken.toTextStyle() = TextStyle(
    fontFamily = when (font) {
        FontRole.TEXT -> Sora
        FontRole.NUMBER -> JetBrainsMono
    },
    fontWeight = FontWeight(weight),
    fontSize = sizeSp.sp,
    lineHeight = (sizeSp * lineHeight).sp,
    letterSpacing = letterSpacingEm.em,
)

/** Prices, distances and times; not part of Material's type scale. */
@Immutable
data class NumberStyles(val large: TextStyle, val medium: TextStyle, val small: TextStyle)

internal val WhichrideNumberStyles = with(WhichrideTokens.Type) {
    NumberStyles(numberLarge.toTextStyle(), numberMedium.toTextStyle(), numberSmall.toTextStyle())
}

/** The design has no display sizes; Material's display slots use our largest headline. */
internal val WhichrideTypography: Typography = with(WhichrideTokens.Type) {
    Typography(
        displayLarge = headlineLarge.toTextStyle(),
        displayMedium = headlineLarge.toTextStyle(),
        displaySmall = headlineLarge.toTextStyle(),
        headlineLarge = headlineLarge.toTextStyle(),
        headlineMedium = headlineMedium.toTextStyle(),
        headlineSmall = headlineSmall.toTextStyle(),
        titleLarge = titleLarge.toTextStyle(),
        titleMedium = titleMedium.toTextStyle(),
        titleSmall = titleSmall.toTextStyle(),
        bodyLarge = bodyLarge.toTextStyle(),
        bodyMedium = bodyMedium.toTextStyle(),
        bodySmall = bodySmall.toTextStyle(),
        labelLarge = labelLarge.toTextStyle(),
        labelMedium = labelMedium.toTextStyle(),
        labelSmall = labelSmall.toTextStyle(),
    )
}
