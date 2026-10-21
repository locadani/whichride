package io.github.locadani.whichride.server

import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Trip
import java.io.IOException
import java.security.MessageDigest
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.coroutines.executeAsync

data class Place(val label: String, val at: Coordinate)

@Serializable private data class CoordinateDto(@SerialName("lat_e7") val latE7: Int, @SerialName("lon_e7") val lonE7: Int)
@Serializable private data class RouteBody(val from: CoordinateDto, val to: CoordinateDto)
@Serializable private data class SearchBody(val text: String, val near: CoordinateDto? = null)
@Serializable private data class ExpandBody(val url: String)
@Serializable private data class TripDto(val metres: Long, val seconds: Long)
@Serializable private data class PlaceDto(val label: String, val at: CoordinateDto)
@Serializable private data class PlacesDto(val places: List<PlaceDto>)
@Serializable private data class ErrorDto(val error: String)
@Serializable data class NonceBody(val nonce: String)

private fun Coordinate.dto() = CoordinateDto(latE7, lonE7)
private fun CoordinateDto.core() = Coordinate(latE7, lonE7)

@Serializable private data class RegistrationDto(@SerialName("install_id") val installId: String, val token: String)
@Serializable private data class ReportOutcomeDto(val report: String? = null, val status: String)
@Serializable private data class StatusDto(val status: String)

/** The server's answer to a report: the report it created or joined, and its status (`pending`, `accepted`, `current`). */
data class ReportOutcome(val reportId: String?, val status: String)

private val lenient = Json { ignoreUnknownKeys = true }

fun readReportOutcome(text: String): ReportOutcome =
    lenient.decodeFromString<ReportOutcomeDto>(text).let { ReportOutcome(it.report, it.status) }

fun readStatus(text: String): String = lenient.decodeFromString<StatusDto>(text).status

/** Lowercase hex SHA-256 of a signed write's integrity binding (decision 44). */
fun writeHash(method: String, path: String, installId: String, body: String): String = sha256Hex("$method $path\n$installId\n$body")

fun registrationHash(body: String): String = sha256Hex("POST /v1/installs\n$body")

private fun sha256Hex(text: String) =
    MessageDigest.getInstance("SHA-256").digest(text.toByteArray()).joinToString("") { "%02x".format(it) }

/**
 * The whichride server's HTTP API (crates/server/README.md). Coordinates always travel in
 * request bodies, never in URLs (§9). `baseUrl` empty = this build has no server (R9).
 */
class ServerApi(baseUrl: String, private val http: OkHttpClient) {
    private val baseUrl = baseUrl.trimEnd('/')
    private val json = Json { ignoreUnknownKeys = true; explicitNulls = false }

    suspend fun catalogue(): ApiResult<String> = call({ Request.Builder().url(url("/v1/catalogue")).get() }) { it }

    suspend fun register(body: String, integrityToken: String): ApiResult<Install> =
        call({ post("/v1/installs", body).header("X-Integrity-Token", integrityToken) }) {
            json.decodeFromString<RegistrationDto>(it).let { dto -> Install(dto.installId, dto.token) }
        }

    /** A report, vote or confirmation: bearer + a Play Integrity token for these exact bytes (decision 44). */
    suspend fun <T> write(path: String, body: String, bearer: String, integrityToken: String, read: (String) -> T): ApiResult<T> =
        call({ authed(path, body, bearer).header("X-Integrity-Token", integrityToken) }, read)

    suspend fun route(bearer: String, from: Coordinate, to: Coordinate): ApiResult<Trip> =
        call({ authed("/v1/route", json.encodeToString(RouteBody(from.dto(), to.dto())), bearer) }) {
            val trip = json.decodeFromString<TripDto>(it)
            Trip(trip.metres.toUInt(), trip.seconds.toUInt())
        }

    suspend fun geocode(bearer: String, text: String, near: Coordinate?) = search("/v1/geocode", bearer, text, near)

    suspend fun autocomplete(bearer: String, text: String, near: Coordinate?) = search("/v1/autocomplete", bearer, text, near)

    /** The expanded destination as the server's JSON; the core decodes it. */
    suspend fun expand(bearer: String, shortLink: String): ApiResult<String> =
        call({ authed("/v1/expand", json.encodeToString(ExpandBody(shortLink)), bearer) }) { it }

    private suspend fun search(path: String, bearer: String, text: String, near: Coordinate?): ApiResult<List<Place>> =
        call({ authed(path, json.encodeToString(SearchBody(text, near?.dto())), bearer) }) { body ->
            json.decodeFromString<PlacesDto>(body).places.map { Place(it.label, it.at.core()) }
        }

    private fun url(path: String) = "$baseUrl$path"

    private fun post(path: String, body: String) =
        Request.Builder().url(url(path)).post(body.toRequestBody(JSON))

    private fun authed(path: String, body: String, bearer: String) = post(path, body).header("Authorization", "Bearer $bearer")

    /** `request` is built only when a server is configured (an empty base URL is no URL at all). */
    private suspend fun <T> call(request: () -> Request.Builder, read: (String) -> T): ApiResult<T> {
        if (baseUrl.isEmpty()) return ApiResult.Unavailable(Why.NOT_CONFIGURED)
        return try {
            http.newCall(request().build()).executeAsync().use { response ->
                val body = response.body.string()
                when {
                    response.isSuccessful -> runCatching { ApiResult.Ok(read(body)) }
                        .getOrElse { ApiResult.Refused(response.code, "unreadable") }
                    response.code == 429 || response.code >= 500 -> ApiResult.Unavailable(Why.BUSY)
                    else -> ApiResult.Refused(
                        response.code,
                        runCatching { json.decodeFromString<ErrorDto>(body).error }.getOrDefault("unknown"),
                    )
                }
            }
        } catch (_: IOException) {
            ApiResult.Unavailable(Why.OFFLINE)
        } catch (_: IllegalArgumentException) { // a malformed base URL
            ApiResult.Unavailable(Why.NOT_CONFIGURED)
        }
    }

    companion object {
        private val JSON = "application/json".toMediaType()
    }
}
