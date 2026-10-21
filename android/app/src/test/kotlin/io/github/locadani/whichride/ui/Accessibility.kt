package io.github.locadani.whichride.ui

import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsNode
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.SemanticsNodeInteractionsProvider
import androidx.compose.ui.test.isRoot
import androidx.compose.ui.unit.dp
import org.junit.Assert.assertTrue

/**
 * Every control on screen has a name TalkBack can read and a touch area of at least 48 × 48 dp
 * (spec §11, plan R18).
 */
fun SemanticsNodeInteractionsProvider.assertAccessible() {
    val root = onNode(isRoot()).fetchSemanticsNode()
    val controls = root.descendants().filter { node ->
        val config = node.config
        SemanticsActions.OnClick in config || SemanticsProperties.ToggleableState in config || SemanticsActions.SetText in config
    }
    assertTrue("no controls found", controls.isNotEmpty())
    for (node in controls) {
        val config = node.config
        val name = config.getOrNull(SemanticsProperties.ContentDescription)?.joinToString()
            ?: config.getOrNull(SemanticsProperties.Text)?.joinToString()
            ?: config.getOrNull(SemanticsProperties.EditableText)?.text
        assertTrue("control without a name: $config", !name.isNullOrBlank())
        val minimum = with(node.layoutInfo.density) { 48.dp.toPx() } - 0.5f
        // Touch bounds include Material's 48 dp expansion of smaller icon buttons, but are clipped
        // to 0 × 0 for controls scrolled out of view; the layout size covers those.
        val touch = node.touchBoundsInRoot
        val width = maxOf(touch.width, node.size.width.toFloat())
        val height = maxOf(touch.height, node.size.height.toFloat())
        assertTrue("\"$name\" is $width × $height px, below 48 dp", width >= minimum && height >= minimum)
    }
}

private fun SemanticsNode.descendants(): List<SemanticsNode> = children.flatMap { listOf(it) + it.descendants() }
