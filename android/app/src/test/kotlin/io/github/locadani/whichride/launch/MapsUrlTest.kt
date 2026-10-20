package io.github.locadani.whichride.launch

import io.github.locadani.whichride.core.Coordinate
import org.junit.Assert.assertEquals
import org.junit.Test

class MapsUrlTest {
    private val alex = Coordinate(latE7 = 525_219_000, lonE7 = 134_132_000)
    private val neun = Coordinate(latE7 = 525_022_870, lonE7 = 134_316_050)

    @Test fun degreesAreExactWithoutTrailingZeros() {
        assertEquals("52.5219", degrees(525_219_000))
        assertEquals("-0.5", degrees(-5_000_000))
        assertEquals("-0.0000001", degrees(-1))
        assertEquals("0", degrees(0))
        assertEquals("-180", degrees(-1_800_000_000))
    }

    @Test fun directionsBetweenTwoPoints() = assertEquals(
        "https://www.google.com/maps/dir/?api=1&origin=52.5219,13.4132&destination=52.502287,13.431605&travelmode=driving",
        mapsUrl(alex, MapsTarget.At(neun)),
    )

    @Test fun withoutAStartMapsUsesTheCurrentLocation() = assertEquals(
        "https://www.google.com/maps/dir/?api=1&destination=52.502287,13.431605&travelmode=driving",
        mapsUrl(null, MapsTarget.At(neun)),
    )

    @Test fun aPlaceNameIsEncoded() = assertEquals(
        "https://www.google.com/maps/dir/?api=1&origin=52.5219,13.4132&destination=Markthalle%20Neun%2C%20Berlin&travelmode=driving",
        mapsUrl(alex, MapsTarget.Query("Markthalle Neun, Berlin")),
    )

    @Test fun anUnresolvedShortLinkOpensAsItIs() = assertEquals(
        "https://maps.app.goo.gl/x7Qa2Lm",
        mapsUrl(alex, MapsTarget.Link("https://maps.app.goo.gl/x7Qa2Lm")),
    )
}
