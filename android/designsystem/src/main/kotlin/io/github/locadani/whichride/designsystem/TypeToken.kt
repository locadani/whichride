package io.github.locadani.whichride.designsystem

/** Which bundled font a text style uses: Sora for text, JetBrains Mono for prices, distances and times. */
internal enum class FontRole { TEXT, NUMBER }

/** One text style from design/tokens.json; [lineHeight] is a multiple of [sizeSp]. */
internal data class TypeToken(
    val font: FontRole,
    val sizeSp: Float,
    val weight: Int,
    val lineHeight: Float,
    val letterSpacingEm: Float,
)
