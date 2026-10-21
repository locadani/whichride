package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.crowd.Answer
import io.github.locadani.whichride.crowd.Crowd
import io.github.locadani.whichride.server.ReportOutcome
import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Destination
import io.github.locadani.whichride.core.Prices
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.launch.Launcher
import io.github.locadani.whichride.options.ChoiceStore
import io.github.locadani.whichride.personal.Personal
import io.github.locadani.whichride.personal.PersonalStore
import io.github.locadani.whichride.prices.LoadedPrices
import io.github.locadani.whichride.prices.PriceLoader
import io.github.locadani.whichride.prices.PriceSource
import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.Place
import io.github.locadani.whichride.server.Routes
import io.github.locadani.whichride.server.Why
import java.io.File
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.coroutines.CompletableDeferred

val SEED: String = File("../../crates/server/catalogue.json").readText()
val ALEX = Coordinate(525_219_000, 134_132_000)
val NEUN = Coordinate(525_022_870, 134_316_050)
val MAUERPARK = Coordinate(525_430_000, 134_030_000)
val TRIP = Trip(7_400u, 1_320u)

class FakeRoutes : Routes {
    var route: ApiResult<Trip> = ApiResult.Ok(TRIP)
    var geocode: ApiResult<List<Place>> = ApiResult.Ok(listOf(Place("Markthalle Neun, Eisenbahnstraße 42, Berlin", NEUN)))
    var autocomplete: ApiResult<List<Place>> = ApiResult.Ok(listOf(Place("Mauerpark, Berlin", NEUN)))
    var expand: ApiResult<Destination> = ApiResult.Unavailable(Why.OFFLINE)
    /** When set, `route` waits for it, so a test can replace a trip that is still routing. */
    var routeGate: CompletableDeferred<Unit>? = null
    val routed = mutableListOf<Pair<Coordinate, Coordinate>>()
    val autocompleted = mutableListOf<String>()

    override suspend fun route(from: Coordinate, to: Coordinate): ApiResult<Trip> {
        routeGate?.await()
        routed += from to to
        return route
    }
    override suspend fun geocode(text: String, near: Coordinate?) = geocode
    override suspend fun autocomplete(text: String, near: Coordinate?) = autocomplete.also { autocompleted += text }
    override suspend fun expand(shortLink: String) = expand
}

class FakeLauncher : Launcher {
    val providers = mutableListOf<String>()
    val urls = mutableListOf<String>()
    override fun openProvider(androidPackage: String) { providers += androidPackage }
    override fun openUrl(url: String) { urls += url }
    var languageSettings = 0
    override fun openAppLanguageSettings() { languageSettings++ }
}

class FakeStore(var choices: Set<Choice> = emptySet()) : ChoiceStore {
    override fun load() = choices
    override fun save(choices: Set<Choice>) { this.choices = choices }
}

class SeedPrices : PriceLoader {
    override suspend fun offline() = LoadedPrices(Prices.fromSeed(SEED), PriceSource.BUILT_IN)
    override suspend fun load() = offline()
}

class FakePersonalStore(var saved: Personal = Personal()) : PersonalStore {
    override fun load() = saved
    override fun save(personal: Personal) { saved = personal }
}

/** 2026-10-20 12:00 UTC, as `now` in the ViewModel tests. */
const val NOW = 1_792_497_600_000L
val TODAY: Long = java.time.LocalDate.of(2026, 10, 20).toEpochDay()
const val COORDINATE_LINK = "https://www.google.com/maps/search/?api=1&query=52.502287,13.431605"

class FakeCrowd : Crowd {
    var reportResult: ApiResult<ReportOutcome> = ApiResult.Ok(ReportOutcome("22222222-2222-4222-8222-222222222222", "pending"))
    var voteResult: ApiResult<Unit> = ApiResult.Ok(Unit)
    val reports = mutableListOf<String>()
    val votes = mutableListOf<Pair<String, Answer>>()
    val confirmations = mutableListOf<Pair<String, Answer>>()
    override suspend fun report(body: String) = reportResult.also { reports += body }
    override suspend fun vote(reportId: String, answer: Answer) = voteResult.also { votes += reportId to answer }
    override suspend fun confirm(offerId: String, answer: Answer) = voteResult.also { confirmations += offerId to answer }
}

const val PENDING_REPORT = "11111111-2222-4333-8444-555555555555"
const val PROPOSAL_REPORT = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee"
const val CARD_89 = """{"label":"Standard","kind":"standard","base_fee":100,"per_km":89,"per_minute":0,"km_rounding":"started_unit","minute_rounding":"started_unit"}"""

/** A `GET /v1/catalogue` answer over the seed: MILES S has a pending report, never confirmed. */
fun serverCatalogue(
    offers: String = """[{"offer":"miles.s","confirmed_on":null,"locked":false,"pending":{"report":"$PENDING_REPORT","cards":[$CARD_89]}}]""",
    proposals: String = "[]",
) = """{"catalogue":$SEED,"offers":$offers,"proposals":$proposals}"""

/** Prices as if the server answered: cached first, then from the server. */
class ServerPrices(private val json: String = serverCatalogue()) : PriceLoader {
    var loads = 0
    override suspend fun offline() = LoadedPrices(Prices.fromServerResponse(json), PriceSource.CACHED)
    override suspend fun load(): LoadedPrices {
        loads++
        return LoadedPrices(Prices.fromServerResponse(json), PriceSource.SERVER)
    }
}

/** The seed without SIXT share. */
fun String.replaceSixt(): String {
    val catalogue = kotlinx.serialization.json.Json.parseToJsonElement(this).jsonObject
    val providers = catalogue["providers"]!!.jsonArray.filter { it.jsonObject["id"]!!.jsonPrimitive.content != "sixt" }
    return kotlinx.serialization.json.JsonObject(mapOf("providers" to kotlinx.serialization.json.JsonArray(providers))).toString()
}