package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.personal.AmountOff
import io.github.locadani.whichride.personal.DealField
import io.github.locadani.whichride.personal.DealForm
import io.github.locadani.whichride.personal.DealKind
import io.github.locadani.whichride.personal.Personal
import io.github.locadani.whichride.personal.RefuelCreditDeal
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class PersonalFlowTest {
    private val dispatcher = StandardTestDispatcher()
    private val launcher = FakeLauncher()
    private var personal = FakePersonalStore()

    @Before fun main() = Dispatchers.setMain(dispatcher)
    @After fun reset() = Dispatchers.resetMain()

    private fun TestScope.trip(): CompareViewModel = CompareViewModel(
        routes = FakeRoutes(),
        location = { ALEX },
        launcher = launcher,
        store = FakeStore(),
        priceLoader = SeedPrices(),
        hasLocationPermission = { true },
        personal = personal,
        crowd = FakeCrowd(),
        today = { TODAY },
        newId = { "new" },
        now = { NOW },
    ).also {
        advanceUntilIdle()
        it.onShared(COORDINATE_LINK)
        advanceUntilIdle()
    }

    private fun CompareViewModel.cheapest(provider: String) =
        state.value.quotes.filter { it.providerId == provider }.minOf { it.totalCents }

    private fun CompareViewModel.add(choice: Choice, provider: String) {
        addCars(); pickProvider(provider); toggle(choice); commitCars()
    }

    @Test fun `a saved discount lowers that provider's prices only`() = runTest(dispatcher) {
        val vm = trip()
        val bolt = vm.cheapest("bolt")
        val miles = vm.cheapest("miles")
        vm.openSettings(); vm.openDeals(); vm.newDeal()
        vm.onDealForm(DealForm(providerId = "bolt", kind = DealKind.PERCENT, amount = "50"))
        vm.saveDeal()
        assertEquals(Screen.Deals, vm.state.value.screen)
        assertEquals((bolt + 1) / 2, vm.cheapest("bolt"))
        assertEquals(miles, vm.cheapest("miles"))
        assertEquals(1, personal.saved.deals.size)
    }

    @Test fun `an invalid discount keeps the form open with its fields marked`() = runTest(dispatcher) {
        val vm = trip()
        vm.openSettings(); vm.openDeals(); vm.newDeal()
        vm.onDealForm(DealForm(providerId = null, kind = DealKind.PERCENT, amount = "x"))
        vm.saveDeal()
        assertEquals(Screen.DealEditor, vm.state.value.screen)
        assertEquals(setOf(DealField.PROVIDER, DealField.AMOUNT), vm.state.value.dealForm!!.invalid)
        assertTrue(personal.saved.deals.isEmpty())
    }

    @Test fun `open uses the once discount of that provider only`() = runTest(dispatcher) {
        personal = FakePersonalStore(Personal(deals = listOf(AmountOff("m", "miles", 100, once = true, until = null))))
        val vm = trip()
        vm.add(Choice.AnyOf("bolt"), "bolt")
        vm.openChosen()
        assertEquals(listOf("ee.mtakso.client"), launcher.providers)
        assertTrue(vm.state.value.usedOnce.isEmpty())
        vm.add(Choice.AnyOf("miles"), "miles")
        vm.select(Choice.AnyOf("miles"))
        vm.openChosen()
        assertEquals("com.driveby.app", launcher.providers.last())
        assertEquals(listOf("m"), vm.state.value.usedOnce)
        assertTrue((personal.saved.deals.single() as AmountOff).used)
    }

    @Test fun `undo restores a used discount`() = runTest(dispatcher) {
        personal = FakePersonalStore(Personal(deals = listOf(AmountOff("m", "miles", 100, once = true, until = null))))
        val vm = trip()
        val discounted = vm.cheapest("miles")
        vm.add(Choice.AnyOf("miles"), "miles")
        vm.openChosen()
        assertEquals(discounted + 100, vm.cheapest("miles"))
        vm.undoOnce()
        assertFalse((personal.saved.deals.single() as AmountOff).used)
        assertTrue(vm.state.value.usedOnce.isEmpty())
        assertEquals(discounted, vm.cheapest("miles"))
    }

    @Test fun `the refuel switch is per trip`() = runTest(dispatcher) {
        personal = FakePersonalStore(Personal(deals = listOf(RefuelCreditDeal("f", "miles", 300))))
        val vm = trip()
        val miles = vm.cheapest("miles")
        vm.setRefuel(true)
        assertEquals(miles - 300, vm.cheapest("miles"))
        vm.onShared(COORDINATE_LINK)
        advanceUntilIdle()
        assertFalse(vm.state.value.refuel)
        assertEquals(miles, vm.cheapest("miles"))
    }

    @Test fun `the chosen app opens the provider`() = runTest(dispatcher) {
        val vm = trip()
        vm.chooseApp("sixt", "com.sixt.reservation")
        assertEquals(mapOf("sixt" to "com.sixt.reservation"), personal.saved.apps)
        vm.add(Choice.AnyOf("sixt"), "sixt")
        vm.openChosen()
        assertEquals("com.sixt.reservation", launcher.providers.last())
        vm.pickProvider("sixt")
        vm.checkIn("sixt")
        assertEquals("com.sixt.reservation", launcher.providers.last())
    }

    @Test fun `deleting and toggling a deal`() = runTest(dispatcher) {
        personal = FakePersonalStore(Personal(deals = listOf(AmountOff("m", "miles", 100, once = false, until = null))))
        val vm = trip()
        val discounted = vm.cheapest("miles")
        vm.toggleDeal("m")
        assertEquals(discounted + 100, vm.cheapest("miles"))
        vm.openSettings(); vm.openDeals(); vm.editDeal("m")
        assertEquals("1", vm.state.value.dealForm!!.amount)
        vm.deleteDeal()
        assertTrue(personal.saved.deals.isEmpty())
        assertEquals(Screen.Deals, vm.state.value.screen)
    }
}
