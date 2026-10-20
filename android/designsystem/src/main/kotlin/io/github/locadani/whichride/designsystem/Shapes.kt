package io.github.locadani.whichride.designsystem

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Shapes

/** Canvas radii: 10 logo tile, 14 rows and inputs, 16 buttons, 20 cards, 28 bottom sheets. */
internal val WhichrideShapes = with(WhichrideTokens.Radius) {
    Shapes(
        extraSmall = RoundedCornerShape(sm),
        small = RoundedCornerShape(md),
        medium = RoundedCornerShape(lg),
        large = RoundedCornerShape(xl),
        extraLarge = RoundedCornerShape(xxl),
    )
}
