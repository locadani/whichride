package io.github.locadani.whichride.ui

import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.swipeLeft
import androidx.compose.ui.test.swipeRight
import androidx.compose.ui.test.hasScrollAction
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.performScrollToNode
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.compare.Start
import io.github.locadani.whichride.compare.Target
import io.github.locadani.whichride.compare.TripEdit
import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.server.Place
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.AppInfo
import io.github.locadani.whichride.core.CardKind
import io.github.locadani.whichride.core.PriceLine
import io.github.locadani.whichride.core.Rounding
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

fun quote(provider: String, offer: String, name: String, cents: Long, days: UInt? = 2u, lines: List<PriceLine> = emptyList()) =
    Quote(provider, offer, name, "Standard", cents, false, days, false, lines)

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
    private var tripToggled = 0

    private fun show(state: UiState) = compose.setContent {
        WhichrideTheme {
            ResultScreen(state, ResultActions(onToggleTrip = { tripToggled++ }, onAddCars = { added++ }, onOpen = { opened++ }))
        }
    }

    @Test fun priceDetailsAreHiddenUntilTheChevronIsTapped() {
        val lines = listOf(
            PriceLine.BaseFee(CardKind.STANDARD, 100),
            PriceLine.Distance(8_000uL, 79, 0u, Rounding.STARTED_UNIT, 632),
            PriceLine.PercentDiscount(1_000u.toUShort(), -73),
            PriceLine.AmountDiscount(-200),
        )
        val miles = RankedOption(Choice.Offer("miles.s"), quote("miles", "miles.s", "S", 459, lines = lines))
        show(resultState(listOf(miles)))
        compose.onNodeWithText("Unlock fee").assertDoesNotExist()
        compose.onNodeWithContentDescription("Show price details for MILES S").performClick()
        compose.onNodeWithText("Unlock fee").assertExists()
        compose.onNodeWithText("€1.00").assertExists()
        compose.onNodeWithText("8 km × €0.79").assertExists()
        compose.onNodeWithText("Discount −10 %").assertExists()
        compose.onNodeWithText("−€0.73").assertExists()
        compose.onNodeWithText("−€2.00").assertExists()
        compose.onNodeWithText("Every started km counts").assertExists()
        compose.onNodeWithContentDescription("Hide price details for MILES S").performClick()
        compose.onNodeWithText("Unlock fee").assertDoesNotExist()
    }

    @Test fun eachOptionOpensOnItsOwn() {
        val bolt = RankedOption(Choice.Offer("bolt.picanto"), quote("bolt", "bolt.picanto", "Kia Picanto", 790,
            lines = listOf(PriceLine.Time(1_320uL, 11, 0u, Rounding.STARTED_UNIT, 242))))
        val miles = RankedOption(Choice.Offer("miles.s"), quote("miles", "miles.s", "S", 849,
            lines = listOf(PriceLine.Distance(7_400uL, 79, 200u, Rounding.EXACT, 0), PriceLine.Minimum(849, 849))))
        show(resultState(listOf(bolt, miles)))
        compose.onNodeWithContentDescription("Show price details for MILES S").performClick()
        compose.onNodeWithText("7.4 km × €0.79").assertExists()
        compose.onNodeWithText("first 200 km included").assertExists()
        compose.onNodeWithText("Minimum charge €8.49").assertExists()
        compose.onNodeWithText("22 min × €0.11").assertDoesNotExist()
        compose.onNodeWithContentDescription("Show price details for Bolt Drive Kia Picanto").performClick()
        compose.onNodeWithText("22 min × €0.11").assertExists()
        compose.onNode(hasScrollAction()).performScrollToNode(hasText("7.4 km × €0.79"))
        compose.onNodeWithText("7.4 km × €0.79").assertExists()
    }

    private fun showRemovable(removed: MutableList<Choice>) {
        val polo = RankedOption(Choice.Offer("bolt.polo"), QUOTES[2])
        val miles = RankedOption(Choice.Offer("miles.s"), QUOTES[1])
        compose.setContent { WhichrideTheme { ResultScreen(resultState(listOf(miles, polo)), ResultActions(onRemove = { removed += it })) } }
    }

    @Test fun swipingAnOptionLeftRemovesIt() {
        val removed = mutableListOf<Choice>()
        showRemovable(removed)
        compose.onNodeWithText("Bolt Drive VW Polo").performTouchInput { swipeLeft() }
        compose.waitForIdle()
        assertEquals(listOf<Choice>(Choice.Offer("bolt.polo")), removed)
    }

    @Test fun aCarPutBackByUndoIsShownAgain() {
        val polo = RankedOption(Choice.Offer("bolt.polo"), QUOTES[2])
        val miles = RankedOption(Choice.Offer("miles.s"), QUOTES[1])
        var ranked by mutableStateOf(listOf(miles, polo))
        compose.setContent {
            WhichrideTheme { ResultScreen(resultState(ranked), ResultActions(onRemove = { c -> ranked = ranked.filter { it.choice != c } })) }
        }
        compose.onNodeWithText("Bolt Drive VW Polo").performTouchInput { swipeLeft() }
        compose.waitForIdle()
        compose.onNodeWithText("Bolt Drive VW Polo").assertDoesNotExist()
        ranked = listOf(miles, polo)
        compose.waitForIdle()
        compose.onNodeWithText("Bolt Drive VW Polo").assertIsDisplayed()
    }

    @Test fun swipingRightDoesNothing() {
        val removed = mutableListOf<Choice>()
        showRemovable(removed)
        compose.onNodeWithText("Bolt Drive VW Polo").performTouchInput { swipeRight() }
        compose.waitForIdle()
        assertEquals(emptyList<Choice>(), removed)
    }

    @Test fun talkBackCanRemoveWithoutSwiping() {
        val removed = mutableListOf<Choice>()
        showRemovable(removed)
        val label = "Remove MILES S"
        // On the option's own row (a radio button), where TalkBack's focus lands.
        val node = compose.onNode(SemanticsMatcher("has $label") {
            it.config.getOrNull(SemanticsActions.CustomActions).orEmpty().any { action -> action.label == label } &&
                it.config.getOrNull(SemanticsProperties.Role) == Role.RadioButton
        }).fetchSemanticsNode()
        compose.runOnUiThread { node.config[SemanticsActions.CustomActions].single { it.label == label }.action() }
        assertEquals(listOf<Choice>(Choice.Offer("miles.s")), removed)
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
        compose.onNodeWithContentDescription("Remove Bolt Drive VW Polo").assertDoesNotExist()
        compose.onNodeWithText("Open MILES").assertIsEnabled().performClick()
        assertEquals(1, opened)
    }

    @Test fun theTripLineShowsKmMinutesAndBuiltInPrices() {
        show(resultState().copy(priceSource = PriceSource.BUILT_IN))
        compose.onNodeWithText("7.4 km · 22 min · built-in prices").assertExists()
    }

    @Test fun closedTripDetailsShowTheLineAndOpenFromTheTitle() {
        show(resultState())
        compose.onNodeWithText("Change start").assertDoesNotExist()
        compose.onNodeWithText("Edit").assertDoesNotExist()
        compose.onNodeWithText("From").assertDoesNotExist()
        compose.onNodeWithContentDescription("Show trip details").performClick()
        assertEquals(1, tripToggled)
    }

    @Test fun openTripDetailsShowStartDestinationAndFieldsOnce() {
        show(resultState().copy(tripEdit = TripEdit(Trip(7_400u, 1_320u))))
        compose.onNodeWithText("7.4 km · 22 min", substring = true).assertDoesNotExist()
        compose.onNodeWithContentDescription("Hide trip details").assertExists()
        compose.onNodeWithText("Your location").assertExists()
        compose.onNodeWithText("Markthalle Neun").assertExists()
        compose.onNodeWithText("7.4").assertExists()
        compose.onNodeWithText("22").assertExists()
        compose.onNodeWithContentDescription("Reset distance to 7.4 km").assertDoesNotExist()
    }

    @Test fun anEditedNumberCanBeReset() {
        var reset = 0
        val edited = TripEdit(Trip(7_400u, 1_320u), minutes = "30")
        compose.setContent {
            WhichrideTheme { ResultScreen(resultState().copy(tripEdit = edited), ResultActions(onResetMinutes = { reset++ })) }
        }
        compose.onNodeWithContentDescription("Reset distance to 7.4 km").assertDoesNotExist()
        compose.onNodeWithContentDescription("Reset time to 22 min").performClick()
        assertEquals(1, reset)
    }

    @Test fun aChosenStartIsShownByName() {
        val start = Start.Chosen(Place("Alexanderplatz, Berlin", Coordinate(525_219_000, 134_132_000)))
        show(resultState().copy(start = start, tripEdit = TripEdit(Trip(7_400u, 1_320u))))
        compose.onNodeWithText("Alexanderplatz").assertExists()
        compose.onNodeWithText("Berlin").assertExists()
        compose.onNodeWithText("Your location").assertDoesNotExist()
    }

    @Test fun invalidTripNumbersSayWhatIsAllowed() {
        show(resultState().copy(tripEdit = TripEdit(Trip(7_400u, 1_320u), km = "abc", kmInvalid = true, minutes = "0", minutesInvalid = true)))
        compose.onNodeWithText("Between 0.1 and 2000 km").assertExists()
        compose.onNodeWithText("Between 1 and 1440 minutes").assertExists()
    }

    @Test fun builtInPricesStayMarkedWhileTheDetailsAreOpen() {
        show(resultState().copy(priceSource = PriceSource.BUILT_IN, tripEdit = TripEdit(Trip(7_400u, 1_320u))))
        compose.onNodeWithText("built-in prices").assertExists()
    }

    @Test fun theTripTitleIsAButton() {
        show(resultState())
        compose.onNode(hasContentDescription("Show trip details") and SemanticsMatcher.expectValue(SemanticsProperties.Role, Role.Button))
            .assertExists()
    }

    @Test fun aTypedTripHasNoMapsButton() {
        show(resultState().copy(target = null))
        compose.onNodeWithText("Your trip").assertExists()
        compose.onNodeWithContentDescription("Open in Google Maps").assertDoesNotExist()
    }

    @Test fun aTypedTripHasNoPlaces() {
        show(resultState().copy(target = null, tripEdit = TripEdit(Trip(7_400u, 1_320u))))
        compose.onNodeWithText("Your trip").assertExists()
        compose.onNodeWithText("From").assertDoesNotExist()
        compose.onNodeWithText("To").assertDoesNotExist()
        compose.onNodeWithText("7.4").assertExists()
    }

    @Test fun `the refuel switch shows only with an active credit`() {
        var refuel: Boolean? = null
        val withCredit = resultState().copy(personal = Personal(deals = listOf(RefuelCreditDeal("f", "miles", 300))))
        compose.setContent { WhichrideTheme { ResultScreen(withCredit, resultActions(onRefuel = { refuel = it })) } }
        compose.onNodeWithText("I'll refuel or charge").performClick()
        assertEquals(true, refuel)
        compose.onNodeWithText("Discounts · 1").assertExists()
    }

    @Test fun `price wrong is offered for accepted offers only`() {
        var reported = false
        val polo = RankedOption(Choice.Offer("bolt.polo"), QUOTES[2])
        compose.setContent { WhichrideTheme { ResultScreen(resultState(listOf(polo)), ResultActions(onReport = { reported = true })) } }
        compose.onNode(hasScrollAction()).performScrollToNode(hasText("Price wrong? Tell us"))
        compose.onNodeWithText("Price wrong? Tell us").performClick()
        assertEquals(true, reported)
    }

    @Test fun `no price wrong for a proposal`() {
        val proposal = RankedOption(Choice.Offer("proposal.x"), quote("bolt", "proposal.x", "Fiat 500e", 700).copy(proposed = true))
        compose.setContent { WhichrideTheme { ResultScreen(resultState(listOf(proposal)), ResultActions()) } }
        compose.onNodeWithText("Price wrong? Tell us").assertDoesNotExist()
        compose.onNodeWithText("added by a user, not confirmed", substring = true).assertExists()
    }
}
