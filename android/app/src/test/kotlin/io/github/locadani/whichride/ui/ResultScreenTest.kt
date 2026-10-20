package io.github.locadani.whichride.ui

import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.compare.Target
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.AppInfo
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.OfferInfo
import io.github.locadani.whichride.core.Options
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.core.Quote
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.launch.MapsTarget
import io.github.locadani.whichride.personal.Personal
import io.github.locadani.whichride.personal.RefuelCreditDeal
import io.github.locadani.whichride.prices.PriceSource
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

fun quote(provider: String, offer: String, name: String, cents: Long, days: UInt? = 2u) =
    Quote(provider, offer, name, "Standard", cents, false, days, false)

val PROVIDERS = listOf(
    ProviderInfo("miles", "MILES", "com.driveby.app", listOf(OfferInfo("miles.s", "S")), listOf(AppInfo("MILES", "com.driveby.app"))),
    ProviderInfo("bolt", "Bolt Drive", "ee.mtakso.client", listOf(OfferInfo("bolt.picanto", "Kia Picanto"), OfferInfo("bolt.polo", "VW Polo")), listOf(AppInfo("Bolt Drive", "ee.mtakso.client"))),
)
val QUOTES = listOf(quote("bolt", "bolt.picanto", "Kia Picanto", 790), quote("miles", "miles.s", "S", 849), quote("bolt", "bolt.polo", "VW Polo", 912))

fun resultState(ranked: List<RankedOption> = emptyList(), cheaper: List<Quote> = emptyList()) = UiState(
    stack = listOf(Screen.Home, Screen.Result),
    target = Target("Markthalle Neun", null, MapsTarget.Query("Markthalle Neun")),
    trip = Trip(7_400u, 1_320u),
    priceSource = PriceSource.SERVER,
    providers = PROVIDERS,
    quotes = QUOTES,
    options = Options(ranked, cheaper),
)

fun resultActions(onRefuel: (Boolean) -> Unit = {}) = ResultActions(onRefuel = onRefuel)

@RunWith(AndroidJUnit4::class)
class ResultScreenTest {
    @get:Rule val compose = createComposeRule()
    private var opened = 0
    private var added = 0
    private var startChanged = 0

    private fun show(state: UiState) = compose.setContent {
        WhichrideTheme {
            ResultScreen(state, ResultActions(onChangeStart = { startChanged++ }, onAddCars = { added++ }, onOpen = { opened++ }))
        }
    }

    @Test fun withoutOptionsOpenIsDisabledAndAddingIsTheWayOn() {
        show(resultState())
        compose.onNodeWithText("Add cars to compare").assertIsNotEnabled()
        compose.onNodeWithText("Which cars are near you?").assertExists()
        compose.onNodeWithText("Add the cars near you").performClick()
        assertEquals(1, added)
    }

    @Test fun theCheapestOptionIsHighlightedAndOpened() {
        val polo = RankedOption(Choice.Offer("bolt.polo"), QUOTES[2])
        val miles = RankedOption(Choice.AnyOf("miles"), QUOTES[1])
        show(resultState(listOf(miles, polo), cheaper = listOf(QUOTES[0])))
        compose.onNodeWithText("CHEAPEST OF YOUR OPTIONS").assertExists()
        compose.onNodeWithText("€8.49").assertExists()
        compose.onNodeWithText("+€0.63").assertExists()
        val cheaper = "1 car you did not add is cheaper: €7.90 (Bolt Drive Kia Picanto)."
        compose.onNode(hasScrollAction()).performScrollToNode(hasText(cheaper))
        compose.onNodeWithText(cheaper).assertExists()
        compose.onNodeWithContentDescription("Remove Bolt Drive VW Polo").assertExists()
        compose.onNodeWithText("Open MILES").assertIsEnabled().performClick()
        assertEquals(1, opened)
    }

    @Test fun theTripLineShowsKmMinutesAndBuiltInPrices() {
        show(resultState().copy(priceSource = PriceSource.BUILT_IN))
        compose.onNodeWithText("7.4 km · 22 min · built-in prices").assertExists()
    }

    @Test fun theStartCanBeChangedFromTheResult() {
        show(resultState())
        compose.onNodeWithText("Change start").performClick()
        assertEquals(1, startChanged)
    }

    @Test fun `the refuel switch shows only with an active credit`() {
        var refuel: Boolean? = null
        val withCredit = resultState().copy(personal = Personal(deals = listOf(RefuelCreditDeal("f", "miles", 300))))
        compose.setContent { WhichrideTheme { ResultScreen(withCredit, resultActions(onRefuel = { refuel = it })) } }
        compose.onNodeWithText("I'll refuel or charge").performClick()
        assertEquals(true, refuel)
        compose.onNodeWithText("Discounts · 1").assertExists()
    }
}
