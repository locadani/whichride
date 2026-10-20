package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.dayOf
import io.github.locadani.whichride.crowd.Answer
import io.github.locadani.whichride.crowd.PricePurpose
import io.github.locadani.whichride.personal.Personal
import io.github.locadani.whichride.personal.overrideCards
import io.github.locadani.whichride.personal.withOverride
import io.github.locadani.whichride.prices.PriceLoader
import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.ReportOutcome
import io.github.locadani.whichride.server.Why
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
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class CrowdFlowTest {
    private val dispatcher = StandardTestDispatcher()
    private val launcher = FakeLauncher()
    private val crowd = FakeCrowd()
    private var personal = FakePersonalStore()
    private var loader: PriceLoader = ServerPrices()

    @Before fun main() = Dispatchers.setMain(dispatcher)
    @After fun reset() = Dispatchers.resetMain()

    private fun TestScope.trip(): CompareViewModel = CompareViewModel(
        routes = FakeRoutes(), location = { ALEX }, launcher = launcher, store = FakeStore(), priceLoader = loader,
        hasLocationPermission = { true }, personal = personal, crowd = crowd, today = { TODAY }, newId = { "new" }, now = { NOW },
    ).also {
        advanceUntilIdle()
        it.onShared(COORDINATE_LINK)
        advanceUntilIdle()
    }

    private fun CompareViewModel.addMilesS() {
        addCars(); pickProvider("miles"); toggle(Choice.Offer("miles.s")); commitCars()
    }

    private fun CompareViewModel.milesS() = state.value.quotes.first { it.offerId == "miles.s" }

    @Test fun `a correction applies to me at once and is sent`() = runTest(dispatcher) {
        val vm = trip()
        vm.addMilesS()
        vm.reportPrice()
        assertEquals(Screen.PriceForm, vm.state.value.screen)
        vm.onPriceForm(vm.state.value.priceForm!!.copy(perKm = "0.50"))
        vm.savePrice()
        assertEquals(Screen.Result, vm.state.value.screen)
        assertTrue(vm.milesS().overridden)
        assertEquals(50L, personal.saved.overrideCards("miles.s")!!.single().perKm)
        advanceUntilIdle()
        assertTrue(crowd.reports.single().contains("\"offer\":\"miles.s\""))
        assertEquals(CrowdMessage.SENT, vm.state.value.crowdMessage)
    }

    @Test fun `a correction without a server still applies to me`() = runTest(dispatcher) {
        crowd.reportResult = ApiResult.Unavailable(Why.OFFLINE)
        val vm = trip()
        vm.addMilesS(); vm.reportPrice()
        vm.onPriceForm(vm.state.value.priceForm!!.copy(perKm = "0,50"))
        vm.savePrice(); advanceUntilIdle()
        assertTrue(vm.milesS().overridden)
        assertEquals(CrowdMessage.NOT_SENT, vm.state.value.crowdMessage)
        assertEquals(1, crowd.reports.size)
    }

    @Test fun `a locked offer keeps my price`() = runTest(dispatcher) {
        crowd.reportResult = ApiResult.Refused(409, "offer_locked")
        val vm = trip()
        vm.addMilesS(); vm.reportPrice()
        vm.onPriceForm(vm.state.value.priceForm!!.copy(perKm = "0,50"))
        vm.savePrice(); advanceUntilIdle()
        assertEquals(CrowdMessage.LOCKED, vm.state.value.crowdMessage)
        assertTrue(vm.milesS().overridden)
    }

    @Test fun `when the server says it is already the price my override goes`() = runTest(dispatcher) {
        crowd.reportResult = ApiResult.Ok(ReportOutcome(null, "current"))
        val vm = trip()
        vm.addMilesS(); vm.reportPrice()
        vm.onPriceForm(vm.state.value.priceForm!!.copy(perKm = "0,50"))
        vm.savePrice(); advanceUntilIdle()
        assertEquals(CrowdMessage.ALREADY_CURRENT, vm.state.value.crowdMessage)
        assertNull(personal.saved.overrideCards("miles.s"))
    }

    @Test fun `unchanged values send nothing`() = runTest(dispatcher) {
        val vm = trip()
        vm.addMilesS(); vm.reportPrice(); vm.savePrice(); advanceUntilIdle()
        assertTrue(crowd.reports.isEmpty())
        assertEquals(Screen.Result, vm.state.value.screen)
    }

    @Test fun `the question follows open and return and a yes votes for the pending report`() = runTest(dispatcher) {
        val vm = trip()
        vm.addMilesS()
        vm.openChosen()
        vm.onResume()
        val question = vm.state.value.question!!
        assertEquals(PENDING_REPORT, question.question.reportId)
        assertEquals(89L, question.question.cards.single().perKm)
        assertEquals(dayOf(NOW / 1_000), personal.saved.lastAskedDay)
        vm.answerYes(); advanceUntilIdle()
        assertEquals(listOf(PENDING_REPORT to Answer.YES), crowd.votes)
        assertEquals(QuestionStep.CONFIRMED, vm.state.value.question!!.step)
        vm.closeQuestion()
        assertNull(vm.state.value.question)
    }

    @Test fun `the question is asked at most once a week and never after check in`() = runTest(dispatcher) {
        personal = FakePersonalStore(Personal(lastAskedDay = dayOf(NOW / 1_000) - 3))
        val vm = trip()
        vm.addMilesS(); vm.openChosen(); vm.onResume()
        assertNull(vm.state.value.question)
        personal.saved = personal.saved.copy(lastAskedDay = null)
        val fresh = trip()
        fresh.addCars(); fresh.pickProvider("miles"); fresh.checkIn("miles"); fresh.onResume()
        assertNull(fresh.state.value.question)
    }

    @Test fun `no question about built-in prices or without a server`() = runTest(dispatcher) {
        loader = SeedPrices()
        val vm = trip()
        vm.addMilesS(); vm.openChosen(); vm.onResume()
        assertNull(vm.state.value.question)
    }

    @Test fun `no then I don't know votes no on the pending report`() = runTest(dispatcher) {
        val vm = trip()
        vm.addMilesS(); vm.openChosen(); vm.onResume()
        vm.answerNo()
        assertEquals(QuestionStep.FIX, vm.state.value.question!!.step)
        assertEquals("0.89", vm.state.value.priceForm!!.perKm.replace(',', '.'))
        vm.dontKnow(); advanceUntilIdle()
        assertEquals(listOf(PENDING_REPORT to Answer.NO), crowd.votes)
        assertNull(vm.state.value.question)
    }

    @Test fun `a correction from the question is a report and my price`() = runTest(dispatcher) {
        val vm = trip()
        vm.addMilesS(); vm.openChosen(); vm.onResume(); vm.answerNo()
        vm.onPriceForm(vm.state.value.priceForm!!.copy(perKm = "0,79"))
        vm.savePrice(); advanceUntilIdle()
        assertEquals(QuestionStep.CORRECTED, vm.state.value.question!!.step)
        assertEquals(79L, personal.saved.overrideCards("miles.s")!!.single().perKm)
        assertEquals(1, crowd.reports.size)
        assertNull(vm.state.value.crowdMessage)
    }

    @Test fun `a current price that is stale is confirmed, not voted`() = runTest(dispatcher) {
        loader = ServerPrices(serverCatalogue(offers = "[]"))
        val vm = trip()
        vm.addMilesS(); vm.openChosen(); vm.onResume()
        assertNull(vm.state.value.question!!.question.reportId)
        vm.answerYes(); advanceUntilIdle()
        assertEquals(listOf("miles.s" to Answer.YES), crowd.confirmations)
    }

    @Test fun `a proposal is sent and the prices reloaded`() = runTest(dispatcher) {
        val server = ServerPrices().also { loader = it }
        val vm = trip()
        vm.addCars(); vm.pickProvider("bolt")
        vm.proposeCar("bolt")
        assertEquals(PricePurpose.Propose("bolt"), vm.state.value.priceForm!!.purpose)
        vm.onPriceForm(vm.state.value.priceForm!!.copy(name = "Fiat 500e", unlock = "1", perKm = "0,25", perMinute = "0,10"))
        vm.savePrice(); advanceUntilIdle()
        assertTrue(crowd.reports.single().contains("\"provider\":\"bolt\""))
        assertEquals(CrowdMessage.PROPOSED, vm.state.value.crowdMessage)
        assertEquals(Screen.Cars("bolt"), vm.state.value.screen)
        assertEquals(2, server.loads)
    }

    @Test fun `a proposal question offers no correction`() = runTest(dispatcher) {
        val proposal = """[{"report":"$PROPOSAL_REPORT","provider":"bolt","name":"Fiat 500e","cards":[$CARD_89]}]"""
        loader = ServerPrices(serverCatalogue(offers = "[]", proposals = proposal))
        val vm = trip()
        vm.addCars(); vm.pickProvider("bolt"); vm.toggle(Choice.Offer("proposal.$PROPOSAL_REPORT")); vm.commitCars()
        vm.openChosen(); vm.onResume()
        assertTrue(vm.state.value.question!!.isProposal)
        vm.answerNo(); advanceUntilIdle()
        assertEquals(listOf(PROPOSAL_REPORT to Answer.NO), crowd.votes)
        assertNull(vm.state.value.question)
    }

    @Test fun `my override goes once the shared price equals it`() = runTest(dispatcher) {
        val shared = io.github.locadani.whichride.core.Prices.fromServerResponse(serverCatalogue()).cards("miles.s")
        personal = FakePersonalStore(Personal().withOverride("miles.s", shared))
        trip()
        assertNull(personal.saved.overrideCards("miles.s"))
        assertNotNull(shared.firstOrNull())
    }
}
