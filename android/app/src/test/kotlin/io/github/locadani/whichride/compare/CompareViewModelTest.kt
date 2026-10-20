package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.Destination
import io.github.locadani.whichride.launch.MapsTarget
import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.Place
import io.github.locadani.whichride.server.Why
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class CompareViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val routes = FakeRoutes()
    private val launcher = FakeLauncher()
    private val store = FakeStore()
    private var here: io.github.locadani.whichride.core.Coordinate? = ALEX
    private var permission = true

    @Before fun main() = Dispatchers.setMain(dispatcher)
    @After fun reset() = Dispatchers.resetMain()

    private fun TestScope.viewModel() = CompareViewModel(
        routes = routes,
        location = { here },
        launcher = launcher,
        store = store,
        priceLoader = SeedPrices(),
        hasLocationPermission = { permission },
        now = { 1_792_497_600_000L },
    ).also { advanceUntilIdle() }

    private val coordinateLink = "https://www.google.com/maps/search/?api=1&query=52.502287,13.431605"

    @Test fun `a shared coordinate link is routed from here and priced`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        val state = vm.state.value
        assertEquals(Screen.Result, state.screen)
        assertEquals(listOf(Screen.Home, Screen.Result), state.stack)
        assertEquals(TRIP, state.trip)
        assertEquals(listOf(ALEX to NEUN), routes.routed)
        assertEquals(9, state.quotes.size)
        assertTrue(state.options!!.ranked.isEmpty())
    }

    @Test fun `short link without server goes to manual entry with the shared name`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared("Markthalle Neun\nhttps://maps.app.goo.gl/x7Qa2Lm")
        advanceUntilIdle()
        val state = vm.state.value
        assertEquals(Screen.ManualTrip, state.screen)
        assertEquals(ManualReason.OFFLINE, state.manualReason)
        assertEquals("Markthalle Neun", state.target!!.label)
        vm.openMaps()
        assertEquals(listOf("https://maps.app.goo.gl/x7Qa2Lm"), launcher.urls)
    }

    @Test fun `an expanded short link is routed`() = runTest(dispatcher) {
        routes.expand = ApiResult.Ok(Destination.Coordinates(NEUN, "Markthalle Neun"))
        val vm = viewModel()
        vm.onShared("https://maps.app.goo.gl/x7Qa2Lm")
        advanceUntilIdle()
        assertEquals(Screen.Result, vm.state.value.screen)
        assertEquals("Markthalle Neun", vm.state.value.target!!.label)
    }

    @Test fun `a busy route service asks for km and minutes`() = runTest(dispatcher) {
        routes.route = ApiResult.Unavailable(Why.BUSY)
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertEquals(ManualReason.BUSY, vm.state.value.manualReason)
        vm.onManualKm("7,4")
        vm.onManualMinutes("22")
        vm.submitManual()
        assertEquals(Screen.Result, vm.state.value.screen)
        assertEquals(TRIP, vm.state.value.trip)
    }

    @Test fun `invalid km and minutes are flagged and nothing is compared`() = runTest(dispatcher) {
        routes.route = ApiResult.Unavailable(Why.BUSY)
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.onManualKm("7.4.1")
        vm.onManualMinutes("0")
        vm.submitManual()
        val state = vm.state.value
        assertEquals(Screen.ManualTrip, state.screen)
        assertTrue(state.manualKmInvalid && state.manualMinutesInvalid)
    }

    @Test fun `no route by car asks for km and minutes`() = runTest(dispatcher) {
        routes.route = ApiResult.Refused(422, "no_route")
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertEquals(ManualReason.NO_ROUTE, vm.state.value.manualReason)
    }

    @Test fun `no location goes to manual entry`() = runTest(dispatcher) {
        here = null
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertEquals(ManualReason.NO_LOCATION, vm.state.value.manualReason)
        assertTrue(routes.routed.isEmpty())
    }

    @Test fun `location permission is asked once and the trip waits for the answer`() = runTest(dispatcher) {
        permission = false
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertTrue(vm.state.value.askLocation)
        assertEquals(Screen.Resolving, vm.state.value.screen)
        permission = true
        vm.onLocationAnswered()
        advanceUntilIdle()
        assertFalse(vm.state.value.askLocation)
        assertEquals(Screen.Result, vm.state.value.screen)
        permission = false
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertFalse("asked only once per session", vm.state.value.askLocation)
    }

    @Test fun `a chosen start is used instead of the location`() = runTest(dispatcher) {
        here = null
        val vm = viewModel()
        vm.changeStart()
        assertEquals(Screen.Start, vm.state.value.screen)
        vm.pickStart(Place("Alexanderplatz, Berlin", ALEX))
        assertEquals(Screen.Home, vm.state.value.screen)
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertEquals(listOf(ALEX to NEUN), routes.routed)
        vm.openMaps()
        assertEquals(
            "https://www.google.com/maps/dir/?api=1&origin=52.5219,13.4132&destination=52.502287,13.431605&travelmode=driving",
            launcher.urls.single(),
        )
    }

    @Test fun `choosing a start after a failed location routes the waiting trip`() = runTest(dispatcher) {
        here = null
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.changeStart()
        vm.pickStart(Place("Alexanderplatz, Berlin", ALEX))
        advanceUntilIdle()
        assertEquals(Screen.Result, vm.state.value.screen)
    }

    @Test fun `a place name is geocoded`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared("https://www.google.com/maps/place/Markthalle+Neun")
        advanceUntilIdle()
        assertEquals(Screen.Result, vm.state.value.screen)
        assertEquals("Markthalle Neun, Eisenbahnstraße 42, Berlin", vm.state.value.target!!.label)
    }

    @Test fun `a place nobody finds goes back home with the reason`() = runTest(dispatcher) {
        routes.geocode = ApiResult.Ok(emptyList())
        val vm = viewModel()
        vm.onQuery("Nowhere street 99")
        vm.submitQuery()
        advanceUntilIdle()
        assertEquals(Screen.Home, vm.state.value.screen)
        assertEquals(InputError.NOT_FOUND, vm.state.value.inputError)
        assertEquals("Nowhere street 99", vm.state.value.query)
    }

    @Test fun `a typed place without the server can still be compared by hand`() = runTest(dispatcher) {
        routes.geocode = ApiResult.Unavailable(Why.NOT_CONFIGURED)
        val vm = viewModel()
        vm.onQuery("Markthalle Neun")
        vm.submitQuery()
        advanceUntilIdle()
        val state = vm.state.value
        assertEquals(ManualReason.OFFLINE, state.manualReason)
        assertEquals(MapsTarget.Query("Markthalle Neun"), state.target!!.maps)
    }

    @Test fun `a link that is not from Google Maps is refused on the home screen`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared("look https://example.com/place")
        assertEquals(Screen.Home, vm.state.value.screen)
        assertEquals(InputError.NOT_A_MAPS_LINK, vm.state.value.inputError)
    }

    @Test fun `shared text without a link is searched like a typed address`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared("Eisenbahnstraße 42, Berlin")
        advanceUntilIdle()
        assertEquals(Screen.Result, vm.state.value.screen)
    }

    @Test fun `a pasted link is offered instead of suggestions`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onQuery("https://maps.app.goo.gl/x7Qa2Lm")
        advanceUntilIdle()
        assertTrue(vm.state.value.linkInQuery)
        assertTrue(routes.autocompleted.isEmpty())
    }

    @Test fun `suggestions wait for a pause and three characters`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onQuery("Ma")
        advanceTimeBy(400)
        vm.onQuery("Mau")
        advanceTimeBy(100)
        vm.onQuery("Maue")
        advanceTimeBy(100)
        vm.onQuery("Mauer")
        advanceUntilIdle()
        assertEquals(listOf("Mauer"), routes.autocompleted)
        assertEquals(listOf(Place("Mauerpark, Berlin", NEUN)), vm.state.value.suggestions)
    }

    @Test fun `suggestions say when the server is unavailable`() = runTest(dispatcher) {
        routes.autocomplete = ApiResult.Unavailable(Why.NOT_CONFIGURED)
        val vm = viewModel()
        vm.onQuery("Mauer")
        advanceUntilIdle()
        assertTrue(vm.state.value.suggestionsUnavailable)
    }

    @Test fun `adding cars ranks them and Open launches the cheapest providers app`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.addCars()
        vm.pickProvider("sixt")
        assertEquals(Screen.Cars("sixt"), vm.state.value.screen)
        vm.toggle(Choice.Offer("sixt.volkswagen-t-roc"))
        vm.commitCars()
        vm.addCars()
        vm.pickProvider("miles")
        vm.toggle(Choice.AnyOf("miles"))
        vm.commitCars()
        val state = vm.state.value
        assertEquals(listOf(Screen.Home, Screen.Result), state.stack)
        assertEquals(2, state.options!!.ranked.size)
        assertEquals(setOf(Choice.Offer("sixt.volkswagen-t-roc"), Choice.AnyOf("miles")), store.choices)
        vm.openChosen()
        val cheapest = state.options!!.ranked.first().quote.providerId
        val pkg = state.providers.single { it.id == cheapest }.androidPackage
        assertEquals(listOf(pkg), launcher.providers)
    }

    @Test fun `the selected option is the one opened`() = runTest(dispatcher) {
        store.choices = setOf(Choice.AnyOf("miles"), Choice.Offer("sixt.volkswagen-t-roc"))
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.select(Choice.Offer("sixt.volkswagen-t-roc"))
        vm.openChosen()
        assertEquals(listOf("com.trafi.whitelabel.bvg"), launcher.providers)
    }

    @Test fun `re-entering a provider shows its cars ticked and removing works`() = runTest(dispatcher) {
        store.choices = setOf(Choice.Offer("bolt.vw-polo"), Choice.AnyOf("miles"))
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.addCars()
        vm.pickProvider("bolt")
        assertEquals(setOf(Choice.Offer("bolt.vw-polo")), vm.state.value.draft)
        assertTrue(vm.back())
        assertTrue(vm.back())
        vm.remove(Choice.AnyOf("miles"))
        assertEquals(setOf(Choice.Offer("bolt.vw-polo")), store.choices)
        assertEquals(1, vm.state.value.options!!.ranked.size)
    }

    @Test fun `check in opens the provider without leaving step 2`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.addCars()
        vm.pickProvider("sixt")
        vm.toggle(Choice.Offer("sixt.volkswagen-t-roc"))
        vm.checkIn("sixt")
        assertEquals(listOf("com.trafi.whitelabel.bvg"), launcher.providers)
        assertEquals(Screen.Cars("sixt"), vm.state.value.screen)
        assertEquals(setOf(Choice.Offer("sixt.volkswagen-t-roc")), vm.state.value.draft)
    }

    @Test fun `a new share replaces the trip and keeps the cars`() = runTest(dispatcher) {
        store.choices = setOf(Choice.AnyOf("bolt"))
        val gate = CompletableDeferred<Unit>()
        routes.routeGate = gate
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertEquals(Screen.Resolving, vm.state.value.screen)
        routes.routeGate = null
        vm.onShared("https://www.google.com/maps/search/?api=1&query=52.5219,13.4132")
        gate.complete(Unit)
        advanceUntilIdle()
        val state = vm.state.value
        assertEquals("only the second destination is routed", listOf(ALEX to ALEX), routes.routed)
        assertEquals(setOf(Choice.AnyOf("bolt")), state.choices)
        assertEquals(1, state.options!!.ranked.size)
    }

    @Test fun `editing km and minutes keeps the result`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.editTrip()
        assertEquals("7.4", vm.state.value.manualKm)
        assertEquals("22", vm.state.value.manualMinutes)
        vm.onManualKm("10")
        vm.submitManual()
        assertEquals(listOf(Screen.Home, Screen.Result), vm.state.value.stack)
        assertEquals(10_000u, vm.state.value.trip!!.metres)
    }

    @Test fun `back walks the stack and cancels routing`() = runTest(dispatcher) {
        routes.routeGate = CompletableDeferred()
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertTrue(vm.back())
        advanceUntilIdle()
        assertEquals(Screen.Home, vm.state.value.screen)
        assertNull(vm.state.value.step)
        assertFalse(vm.back())
    }

    @Test fun `cars expire while the app is in the background`() = runTest(dispatcher) {
        store.choices = setOf(Choice.AnyOf("bolt"))
        val vm = viewModel()
        store.choices = emptySet() // what the real store returns after 15 minutes
        vm.onResume()
        assertTrue(vm.state.value.choices.isEmpty())
    }
}
