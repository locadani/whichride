package io.github.locadani.whichride.server

import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.DataException
import io.github.locadani.whichride.core.Destination
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.core.destinationFromJson

/** What the compare flow needs from the server; the ViewModel's tests fake it. */
interface Routes {
    suspend fun route(from: Coordinate, to: Coordinate): ApiResult<Trip>
    suspend fun geocode(text: String, near: Coordinate?): ApiResult<List<Place>>
    suspend fun autocomplete(text: String, near: Coordinate?): ApiResult<List<Place>>
    suspend fun expand(shortLink: String): ApiResult<Destination>
}

class ServerRoutes(private val api: ServerApi, private val installs: Installs) : Routes {
    override suspend fun route(from: Coordinate, to: Coordinate) = installs.authorized { api.route(it, from, to) }
    override suspend fun geocode(text: String, near: Coordinate?) = installs.authorized { api.geocode(it, text, near) }
    override suspend fun autocomplete(text: String, near: Coordinate?) = installs.authorized { api.autocomplete(it, text, near) }
    override suspend fun expand(shortLink: String): ApiResult<Destination> =
        when (val json = installs.authorized { api.expand(it, shortLink) }) {
            is ApiResult.Ok -> try {
                ApiResult.Ok(destinationFromJson(json.value))
            } catch (_: DataException) {
                ApiResult.Refused(200, "invalid_destination")
            }
            is ApiResult.Unavailable -> json
            is ApiResult.Refused -> json
        }
}
