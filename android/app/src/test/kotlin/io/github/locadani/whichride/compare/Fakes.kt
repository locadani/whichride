package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Destination
import io.github.locadani.whichride.core.Prices
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.launch.Launcher
import io.github.locadani.whichride.options.ChoiceStore
import io.github.locadani.whichride.prices.LoadedPrices
import io.github.locadani.whichride.prices.PriceLoader
import io.github.locadani.whichride.prices.PriceSource
import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.Place
import io.github.locadani.whichride.server.Routes
import io.github.locadani.whichride.server.Why
import java.io.File
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
}

class FakeStore(var choices: Set<Choice> = emptySet()) : ChoiceStore {
    override fun load() = choices
    override fun save(choices: Set<Choice>) { this.choices = choices }
}

class SeedPrices : PriceLoader {
    override suspend fun offline() = LoadedPrices(Prices.fromSeed(SEED), PriceSource.BUILT_IN)
    override suspend fun load() = offline()
}
