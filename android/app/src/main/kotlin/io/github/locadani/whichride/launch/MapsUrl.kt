package io.github.locadani.whichride.launch

import io.github.locadani.whichride.core.Coordinate
import java.net.URLEncoder
import kotlin.math.absoluteValue

/** What Google Maps should show as the destination. */
sealed interface MapsTarget {
    data class At(val at: Coordinate) : MapsTarget
    /** A place name we couldn't turn into coordinates; Maps searches it. */
    data class Query(val text: String) : MapsTarget
    /** A short link we couldn't expand; Maps opens it itself. */
    data class Link(val url: String) : MapsTarget
}

/** Maps URLs (R16): driving directions, from the start when we know it. */
fun mapsUrl(from: Coordinate?, to: MapsTarget): String {
    val destination = when (to) {
        is MapsTarget.Link -> return to.url
        is MapsTarget.At -> pair(to.at)
        is MapsTarget.Query -> URLEncoder.encode(to.text, "UTF-8").replace("+", "%20") // the Charset overload is API 33
    }
    val origin = from?.let { "&origin=${pair(it)}" }.orEmpty()
    return "https://www.google.com/maps/dir/?api=1$origin&destination=$destination&travelmode=driving"
}

private fun pair(c: Coordinate) = "${degrees(c.latE7)},${degrees(c.lonE7)}"

/** 1e-7 degrees as a decimal string, exactly (no floating point), without trailing zeros. */
fun degrees(e7: Int): String {
    val magnitude = e7.toLong().absoluteValue
    val fraction = (magnitude % 10_000_000).toString().padStart(7, '0').trimEnd('0')
    val sign = if (e7 < 0) "-" else ""
    return sign + (magnitude / 10_000_000) + if (fraction.isEmpty()) "" else ".$fraction"
}
