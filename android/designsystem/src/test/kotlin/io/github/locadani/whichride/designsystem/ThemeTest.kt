package io.github.locadani.whichride.designsystem

import androidx.compose.material3.ColorScheme
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.TextUnitType
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ThemeTest {
    /** Every `Color` property of [target], read through the getters Kotlin compiles value classes to. */
    private fun colorsOf(target: Any): Map<String, Color> =
        target.javaClass.methods
            .filter { it.parameterCount == 0 && it.returnType == java.lang.Long.TYPE && it.name.startsWith("get") }
            .associate { it.name.substringAfter("get").substringBefore('-') to Color((it.invoke(target) as Long).toULong()) }

    private fun assertOnlyOurColors(scheme: ColorScheme, colors: WhichrideColors, inversePrimary: Color) {
        val allowed = colorsOf(colors).values.toSet() + inversePrimary
        val roles = colorsOf(scheme)
        assertTrue("ColorScheme exposes no colours: the reflection broke", roles.size >= 25)
        val foreign = roles.filterValues { it !in allowed }
        assertTrue("ColorScheme roles not mapped from tokens: ${foreign.keys}", foreign.isEmpty())
    }

    @Test
    fun `every Material colour role comes from our tokens, in both themes`() {
        val light = WhichrideTokens.LightColors
        val dark = WhichrideTokens.DarkColors
        assertOnlyOurColors(light.toMaterial(inversePrimary = dark.accent), light, dark.accent)
        assertOnlyOurColors(dark.toMaterial(inversePrimary = light.accent), dark, light.accent)
    }

    @Test
    fun `the main roles map as documented`() {
        val c = WhichrideTokens.DarkColors
        val m = c.toMaterial(inversePrimary = WhichrideTokens.LightColors.accent)
        assertEquals(c.accent, m.primary)
        assertEquals(c.onAccent, m.onPrimary)
        assertEquals(c.background, m.background)
        assertEquals(c.surface, m.surfaceContainer)
        assertEquals(c.outline, m.outline)
        assertEquals(c.onSurfaceMuted, m.onSurfaceVariant)
    }

    private fun allTextStyles(): List<Pair<String, TextStyle>> = with(WhichrideTypography) {
        listOf(
            "displayLarge" to displayLarge, "displayMedium" to displayMedium, "displaySmall" to displaySmall,
            "headlineLarge" to headlineLarge, "headlineMedium" to headlineMedium, "headlineSmall" to headlineSmall,
            "titleLarge" to titleLarge, "titleMedium" to titleMedium, "titleSmall" to titleSmall,
            "bodyLarge" to bodyLarge, "bodyMedium" to bodyMedium, "bodySmall" to bodySmall,
            "labelLarge" to labelLarge, "labelMedium" to labelMedium, "labelSmall" to labelSmall,
        )
    } + listOf(
        "numbers.large" to WhichrideNumberStyles.large,
        "numbers.medium" to WhichrideNumberStyles.medium,
        "numbers.small" to WhichrideNumberStyles.small,
    )

    @Test
    fun `text sizes scale with the user's font size`() {
        for ((name, style) in allTextStyles()) {
            assertEquals("$name fontSize", TextUnitType.Sp, style.fontSize.type)
            assertEquals("$name lineHeight", TextUnitType.Sp, style.lineHeight.type)
        }
    }

    @Test
    fun `every style uses a bundled font at a bundled weight`() {
        for ((name, style) in allTextStyles()) {
            assertTrue("$name font", style.fontFamily == Sora || style.fontFamily == JetBrainsMono)
            assertTrue("$name weight ${style.fontWeight}", style.fontWeight!!.weight in WhichrideTokens.Type.weights)
        }
        assertEquals(WhichrideTypography.headlineLarge, WhichrideTypography.displayLarge)
        assertEquals(JetBrainsMono, WhichrideNumberStyles.large.fontFamily)
        assertEquals(36f, WhichrideNumberStyles.large.fontSize.value)
        assertEquals(30f * 1.1f, WhichrideTypography.headlineLarge.lineHeight.value, 0.001f)
    }
}
