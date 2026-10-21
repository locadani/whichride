package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Options
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.core.Question
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.core.Quote
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.crowd.PROPOSAL_PREFIX
import io.github.locadani.whichride.format.formatMinutes
import io.github.locadani.whichride.format.kilometresInput
import io.github.locadani.whichride.format.parseKilometres
import io.github.locadani.whichride.format.parseMinutes
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.launch.MapsTarget
import io.github.locadani.whichride.personal.DealForm
import io.github.locadani.whichride.personal.Personal
import io.github.locadani.whichride.prices.PriceSource
import io.github.locadani.whichride.server.Place

/** Why the user is asked for km and minutes (R25). */
enum class ManualReason { BUSY, OFFLINE, NO_ROUTE, NO_LOCATION }

enum class InputError { NOT_A_MAPS_LINK, NO_DESTINATION, NOT_FOUND, TOO_LONG }

enum class ResolvingStep { FINDING_PLACE, FINDING_LOCATION, ROUTING }

/** Where the trip goes: its name, its position when known, and what Google Maps should open. */
data class Target(val label: String?, val at: Coordinate?, val maps: MapsTarget)

sealed interface Start {
    data object MyLocation : Start
    data class Chosen(val place: Place) : Start
}

enum class QuestionStep { ASK, FIX, CONFIRMED, CORRECTED, NOT_SENT }

/** The §7.2 sheet: what is asked, about which car, and how far the user got. */
data class QuestionState(val question: Question, val providerId: String, val offerName: String, val step: QuestionStep = QuestionStep.ASK) {
    val isProposal: Boolean get() = question.offerId.startsWith(PROPOSAL_PREFIX)
}

/** What happened to a report or proposal the user sent (R11). */
enum class CrowdMessage { SENT, PROPOSED, NOT_SENT, PROPOSAL_NOT_SENT, LOCKED, ALREADY_CURRENT }

data class UiState(
    val stack: List<Screen> = listOf(Screen.Home),
    // Home
    val query: String = "",
    val linkInQuery: Boolean = false,
    val suggestions: List<Place> = emptyList(),
    val suggestionsUnavailable: Boolean = false,
    val inputError: InputError? = null,
    // Start
    val start: Start = Start.MyLocation,
    val startQuery: String = "",
    val startSuggestions: List<Place> = emptyList(),
    // Trip
    val target: Target? = null,
    val step: ResolvingStep? = null,
    val askLocation: Boolean = false,
    val manualReason: ManualReason? = null,
    val manualKm: String = "",
    val manualMinutes: String = "",
    val manualKmInvalid: Boolean = false,
    val manualMinutesInvalid: Boolean = false,
    val trip: Trip? = null,
    /** The trip details under the result's title, while they are open. */
    val tripEdit: TripEdit? = null,
    // Prices and options
    val priceSource: PriceSource? = null,
    val providers: List<ProviderInfo> = emptyList(),
    val quotes: List<Quote> = emptyList(),
    val options: Options? = null,
    val choices: Set<Choice> = emptySet(),
    val selected: Choice? = null,
    val draft: Set<Choice> = emptySet(),
    val carFilter: String = "",
    // Personal (§6)
    val personal: Personal = Personal(),
    /** Epoch day in the phone's time zone, for "expired" labels. */
    val today: Long = 0,
    /** "I'll refuel or charge" for this trip (R6). */
    val refuel: Boolean = false,
    val dealForm: DealForm? = null,
    /** Once discounts just used by Open <provider>, for the Undo snackbar (R5). */
    val usedOnce: List<String> = emptyList(),
    /** The car just swiped away, for the Undo snackbar. */
    val removed: Removed? = null,
    // Crowd (§7, §5.1)
    /** A server is configured, so reports and questions can be sent. */
    val crowdEnabled: Boolean = false,
    val priceForm: PriceForm? = null,
    val question: QuestionState? = null,
    val crowdMessage: CrowdMessage? = null,
    /** Offers the owner locked: no reports, no questions. */
    val lockedOffers: Set<String> = emptySet(),
) {
    val screen: Screen get() = stack.last()
}

/** The option "Price wrong? Tell us" is about: the selected one, else the cheapest. */
fun UiState.reportTarget(): Quote? {
    val ranked = options?.ranked.orEmpty()
    return (ranked.firstOrNull { it.choice == selected } ?: ranked.firstOrNull())?.quote
}

/** Reports are possible for an added, accepted, unlocked offer (R9, R14). */
fun UiState.canReport(): Boolean = reportTarget()?.let { !it.proposed && it.offerId !in lockedOffers } == true

/** A car the user removed from the comparison, and whether it was the selected one. */
data class Removed(val option: RankedOption, val wasSelected: Boolean) {
    val choice get() = option.choice
}

/**
 * The km and minutes being edited in the trip details. `original` is the trip when they were
 * opened; closing them keeps the edited trip, which is then the original next time.
 */
data class TripEdit(
    val original: Trip,
    val km: String = kilometresInput(original.metres),
    val minutes: String = formatMinutes(original.seconds),
    val kmInvalid: Boolean = false,
    val minutesInvalid: Boolean = false,
) {
    val kmEdited get() = parseKilometres(km) != parseKilometres(kilometresInput(original.metres))
    val minutesEdited get() = parseMinutes(minutes) != parseMinutes(formatMinutes(original.seconds))
}

