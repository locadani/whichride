package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Options
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.core.Quote
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.launch.MapsTarget
import io.github.locadani.whichride.personal.DealForm
import io.github.locadani.whichride.personal.Personal
import io.github.locadani.whichride.prices.PriceSource
import io.github.locadani.whichride.server.Place

/** Why the user is asked for km and minutes (R25). */
enum class ManualReason { BUSY, OFFLINE, NO_ROUTE, NO_LOCATION, EDIT }

enum class InputError { NOT_A_MAPS_LINK, NO_DESTINATION, NOT_FOUND, TOO_LONG }

enum class ResolvingStep { FINDING_PLACE, FINDING_LOCATION, ROUTING }

/** Where the trip goes: its name, its position when known, and what Google Maps should open. */
data class Target(val label: String?, val at: Coordinate?, val maps: MapsTarget)

sealed interface Start {
    data object MyLocation : Start
    data class Chosen(val place: Place) : Start
}

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
) {
    val screen: Screen get() = stack.last()
}
