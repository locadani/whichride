package io.github.locadani.whichride.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.geometry.Offset
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.border
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.clearAndSetSemantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.server.Place

private val Space = WhichrideTokens.Space
private val Size = WhichrideTokens.Size

/** The main action of a screen: full width, 56 high; disabled in the canvas's grey (decision 51). */
@Composable
fun PrimaryButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, icon: ImageVector? = null) {
    Button(
        onClick = onClick,
        enabled = enabled,
        modifier = modifier.fillMaxWidth().heightIn(min = Size.controlLg),
        shape = MaterialTheme.shapes.medium,
        colors = ButtonDefaults.buttonColors(
            disabledContainerColor = WhichrideTheme.colors.disabled,
            disabledContentColor = WhichrideTheme.colors.onDisabled,
        ),
    ) {
        icon?.let {
            Icon(it, contentDescription = null)
            Spacer(Modifier.width(Space.xs))
        }
        Text(text, style = MaterialTheme.typography.labelLarge)
    }
}

/** A secondary action, outlined in the muted colour (≥ 3:1, decision 51). */
@Composable
fun SecondaryButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, icon: ImageVector? = null) {
    OutlinedButton(
        onClick = onClick,
        modifier = modifier.fillMaxWidth().heightIn(min = Size.controlMd),
        shape = MaterialTheme.shapes.medium,
        border = BorderStroke(Size.stroke, WhichrideTheme.colors.outline),
        colors = ButtonDefaults.outlinedButtonColors(contentColor = WhichrideTheme.colors.onSurface),
    ) {
        icon?.let {
            Icon(it, contentDescription = null)
            Spacer(Modifier.width(Space.xs))
        }
        Text(text, style = MaterialTheme.typography.labelLarge)
    }
}

/** A card: the canvas's surface panels (R22: not Material's Card). */
@Composable
fun Panel(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    Surface(modifier.fillMaxWidth(), shape = MaterialTheme.shapes.large, color = WhichrideTheme.colors.surface) {
        Column(Modifier.padding(Space.xs), content = content)
    }
}

/** The provider's initial in a circle; silent for TalkBack, as the name is always next to it. */
@Composable
fun Monogram(name: String, large: Boolean = false) {
    Box(
        Modifier.clearAndSetSemantics {}.size(if (large) Size.controlMd else Size.monogram).clip(CircleShape)
            .background(WhichrideTheme.colors.surfaceHighest),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            name.take(1).uppercase(),
            style = if (large) MaterialTheme.typography.titleMedium else MaterialTheme.typography.labelMedium,
            color = WhichrideTheme.colors.onSurface,
        )
    }
}

/** A small accent pill, e.g. "2 added". */
@Composable
fun Badge(text: String) {
    Surface(shape = RoundedCornerShape(WhichrideTokens.Radius.full), color = WhichrideTheme.colors.accent, contentColor = WhichrideTheme.colors.onAccent) {
        Text(text, style = MaterialTheme.typography.labelSmall, modifier = Modifier.padding(horizontal = Space.sm, vertical = Space.xxs))
    }
}

/** Back (or close) and whatever the screen puts next to it. */
@Composable
fun TopBar(onBack: () -> Unit, close: Boolean = false, content: @Composable RowScope.() -> Unit = {}) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = Space.xs, vertical = Space.xxs),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(Space.xxs),
    ) {
        IconButton(onClick = onBack) {
            Icon(
                if (close) Icons.Filled.Close else Icons.AutoMirrored.Filled.ArrowBack,
                contentDescription = stringResource(if (close) R.string.close else R.string.back),
            )
        }
        content()
    }
}

/** A place from autocomplete: the name, then the rest of its address. */
@Composable
fun PlaceRow(place: Place, icon: ImageVector, onClick: () -> Unit) {
    val name = place.label.substringBefore(", ")
    val area = place.label.substringAfter(", ", missingDelimiterValue = "")
    PlainRow(icon = icon, title = name, subtitle = area.ifEmpty { null }, onClick = onClick)
}

@Composable
fun PlainRow(icon: ImageVector, title: String, subtitle: String?, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().clip(MaterialTheme.shapes.small).clickable(onClick = onClick)
            .heightIn(min = Size.controlLg).padding(horizontal = Space.sm, vertical = Space.xs),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        Icon(icon, contentDescription = null, tint = WhichrideTheme.colors.onSurfaceMuted)
        Column {
            Text(title, style = MaterialTheme.typography.titleSmall)
            subtitle?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = WhichrideTheme.colors.onSurfaceMuted) }
        }
    }
}

/** Filled dot, dotted line, hollow square: start, route, destination. */
/** Shared by Home and the trip details, so the two read as the same route. */
@Composable
fun RouteMarks(modifier: Modifier) {
    val colors = WhichrideTheme.colors
    val stroke = WhichrideTokens.Size.strokeStrong
    Column(modifier.fillMaxHeight(), horizontalAlignment = Alignment.CenterHorizontally) {
        Box(Modifier.size(Space.sm).clip(CircleShape).background(colors.accent))
        Canvas(Modifier.weight(1f).width(stroke).padding(vertical = Space.xxs)) {
            val dash = Space.xxs.toPx()
            drawLine(
                colors.outline,
                Offset(size.width / 2, 0f),
                Offset(size.width / 2, size.height),
                strokeWidth = stroke.toPx(),
                pathEffect = PathEffect.dashPathEffect(floatArrayOf(dash, dash)),
            )
        }
        Box(Modifier.size(Space.sm).border(stroke, colors.onSurfaceMuted, RoundedCornerShape(Space.xxs)))
    }
}

