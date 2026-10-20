package io.github.locadani.whichride.compare

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Destination
import io.github.locadani.whichride.core.LinkException
import io.github.locadani.whichride.core.Prices
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.core.parseDestination
import io.github.locadani.whichride.core.sharedLabel
import io.github.locadani.whichride.format.kilometresInput
import io.github.locadani.whichride.format.formatMinutes
import io.github.locadani.whichride.format.parseKilometres
import io.github.locadani.whichride.format.parseMinutes
import io.github.locadani.whichride.launch.Launcher
import io.github.locadani.whichride.launch.MapsTarget
import io.github.locadani.whichride.launch.mapsUrl
import io.github.locadani.whichride.location.LocationSource
import io.github.locadani.whichride.options.ChoiceStore
import io.github.locadani.whichride.options.forProvider
import io.github.locadani.whichride.options.replacingProvider
import io.github.locadani.whichride.options.toggle as toggled
import io.github.locadani.whichride.core.CardKind
import io.github.locadani.whichride.core.RateCard
import io.github.locadani.whichride.core.dayOf
import io.github.locadani.whichride.crowd.Answer
import io.github.locadani.whichride.crowd.Built
import io.github.locadani.whichride.crowd.Crowd
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.crowd.PricePurpose
import io.github.locadani.whichride.crowd.build
import io.github.locadani.whichride.crowd.priceForm
import io.github.locadani.whichride.personal.droppingOverridesEqualTo
import io.github.locadani.whichride.personal.overrideCards
import io.github.locadani.whichride.personal.withOverride
import io.github.locadani.whichride.personal.withoutOverride
import io.github.locadani.whichride.personal.DealForm
import io.github.locadani.whichride.personal.Personal
import io.github.locadani.whichride.personal.PersonalStore
import io.github.locadani.whichride.personal.adjustments
import io.github.locadani.whichride.personal.appFor
import io.github.locadani.whichride.personal.restore
import io.github.locadani.whichride.personal.toForm
import io.github.locadani.whichride.personal.toggled
import io.github.locadani.whichride.personal.useOnce
import io.github.locadani.whichride.personal.validated
import io.github.locadani.whichride.personal.withDeal
import io.github.locadani.whichride.personal.withoutDeal
import java.time.LocalDate
import java.util.Locale
import java.util.UUID
import io.github.locadani.whichride.prices.PriceLoader
import io.github.locadani.whichride.prices.PriceSource
import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.Place
import io.github.locadani.whichride.server.Routes
import io.github.locadani.whichride.server.Why
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * The whole compare flow (spec §4): destination → start → km and minutes → prices → the cars
 * near the user → open an app. Holds the back stack (decision 63); locations stay in memory (§9).
 */
class CompareViewModel(
    private val routes: Routes,
    private val location: LocationSource,
    private val launcher: Launcher,
    private val store: ChoiceStore,
    private val priceLoader: PriceLoader,
    private val hasLocationPermission: () -> Boolean,
    private val personal: PersonalStore,
    private val crowd: Crowd,
    private val crowdEnabled: Boolean = true,
    private val today: () -> Long = { LocalDate.now().toEpochDay() },
    private val newId: () -> String = { UUID.randomUUID().toString() },
    private val now: () -> Long = System::currentTimeMillis,
) : ViewModel() {
    private val mutableState = MutableStateFlow(UiState(choices = store.load(), personal = personal.load(), today = today(), crowdEnabled = crowdEnabled))
    val state: StateFlow<UiState> = mutableState.asStateFlow()

    private var prices: Prices? = null
    private var tripJob: Job? = null
    private var searchJob: Job? = null
    private var startSearchJob: Job? = null
    private var permissionAnswer: CompletableDeferred<Unit>? = null
    private var permissionAsked = false
    /** The last location fix, for search bias and the Maps link; memory only. */
    private var here: Coordinate? = null
    /** The offer the user opened with Open <provider>; the question comes when they return (R12). */
    private var opened: QuestionTarget? = null

    private data class QuestionTarget(val offerId: String, val providerId: String, val offerName: String)

    init {
        viewModelScope.launch {
            usePrices(priceLoader.offline().let { it.prices to it.source })
            val fresh = priceLoader.load()
            if (fresh.source == PriceSource.SERVER) usePrices(fresh.prices to fresh.source)
        }
    }

    // --- Destination input -------------------------------------------------------------

    /** A Google Maps share, a `geo:` link, or a pasted link (spec §4 step 1). */
    fun onShared(text: String) {
        val destination = try {
            parseDestination(text)
        } catch (e: LinkException) {
            if (e is LinkException.NoLink) {
                onQuery(text.trim().take(MAX_QUERY_CHARS))
                return submitQuery()
            }
            tripJob?.cancel()
            mutableState.update { it.copy(stack = listOf(Screen.Home), step = null, inputError = e.toInputError()) }
            return
        }
        startTrip(destination, sharedLabel(text))
    }

    fun onQuery(text: String) {
        val isLink = try {
            parseDestination(text)
            true
        } catch (e: LinkException) {
            e !is LinkException.NoLink
        }
        mutableState.update { it.copy(query = text, linkInQuery = isLink, inputError = null) }
        searchJob?.cancel()
        if (isLink || text.trim().length < MIN_SEARCH_CHARS) {
            mutableState.update { it.copy(suggestions = emptyList(), suggestionsUnavailable = false) }
            return
        }
        searchJob = viewModelScope.launch {
            delay(SEARCH_DELAY_MILLIS)
            val result = routes.autocomplete(text.trim(), startCoordinateIfKnown())
            mutableState.update {
                it.copy(
                    suggestions = (result as? ApiResult.Ok)?.value.orEmpty(),
                    suggestionsUnavailable = result !is ApiResult.Ok,
                )
            }
        }
    }

    fun submitQuery() {
        val text = state.value.query.trim()
        if (text.isEmpty()) return
        if (state.value.linkInQuery) return onShared(text)
        startTrip(Destination.Place(text, null), null)
    }

    fun pickSuggestion(place: Place) = startTrip(Destination.Coordinates(place.at, place.label), null)

    // --- Start -------------------------------------------------------------------------

    fun changeStart() = push(Screen.Start)

    fun onStartQuery(text: String) {
        mutableState.update { it.copy(startQuery = text) }
        startSearchJob?.cancel()
        if (text.trim().length < MIN_SEARCH_CHARS) return mutableState.update { it.copy(startSuggestions = emptyList()) }
        startSearchJob = viewModelScope.launch {
            delay(SEARCH_DELAY_MILLIS)
            val result = routes.autocomplete(text.trim(), here)
            mutableState.update { it.copy(startSuggestions = (result as? ApiResult.Ok)?.value.orEmpty()) }
        }
    }

    fun pickStart(place: Place) = setStart(Start.Chosen(place))

    fun useMyLocation() = setStart(Start.MyLocation)

    private fun setStart(start: Start) {
        mutableState.update { it.copy(start = start, startQuery = "", startSuggestions = emptyList()) }
        pop()
        // A trip already on screen (result or manual entry) is routed again from the new start.
        val target = state.value.target
        if (target?.at != null && (state.value.screen == Screen.ManualTrip || state.value.screen == Screen.Result)) {
            reroute(target)
        }
    }

    fun onLocationAnswered() {
        mutableState.update { it.copy(askLocation = false) }
        permissionAnswer?.complete(Unit)
    }

    // --- Trip --------------------------------------------------------------------------

    private fun startTrip(destination: Destination, label: String?) {
        tripJob?.cancel()
        searchJob?.cancel()
        mutableState.update {
            it.copy(
                stack = listOf(Screen.Home, Screen.Resolving),
                query = "", linkInQuery = false, suggestions = emptyList(), inputError = null,
                target = Target(label, null, MapsTarget.Query(label.orEmpty())),
                step = ResolvingStep.FINDING_PLACE, trip = null, manualReason = null, selected = null, refuel = false,
            )
        }
        tripJob = viewModelScope.launch {
            when (val resolution = resolve(destination, label)) {
                is Resolution.Found -> {
                    mutableState.update { it.copy(target = resolution.target) }
                    route(resolution.target)
                }
                is Resolution.Unrouted -> {
                    mutableState.update { it.copy(target = resolution.target) }
                    manual(resolution.reason)
                }
                is Resolution.Failed -> mutableState.update {
                    it.copy(stack = listOf(Screen.Home), step = null, query = resolution.query, inputError = resolution.error)
                }
            }
        }
    }

    private fun reroute(target: Target) {
        tripJob?.cancel()
        mutableState.update { it.copy(stack = listOf(Screen.Home, Screen.Resolving), step = ResolvingStep.FINDING_LOCATION) }
        tripJob = viewModelScope.launch { route(target) }
    }

    private sealed interface Resolution {
        data class Found(val target: Target) : Resolution
        data class Unrouted(val target: Target, val reason: ManualReason) : Resolution
        data class Failed(val error: InputError, val query: String) : Resolution
    }

    private suspend fun resolve(destination: Destination, label: String?): Resolution = when (destination) {
        is Destination.Coordinates -> Resolution.Found(
            Target(destination.label ?: label, destination.at, MapsTarget.At(destination.at)),
        )
        is Destination.Place -> when (val found = routes.geocode(destination.query, destination.near ?: startCoordinateIfKnown())) {
            is ApiResult.Ok -> found.value.firstOrNull()
                ?.let { Resolution.Found(Target(it.label, it.at, MapsTarget.At(it.at))) }
                ?: Resolution.Failed(InputError.NOT_FOUND, destination.query)
            is ApiResult.Unavailable -> Resolution.Unrouted(
                Target(destination.query, null, MapsTarget.Query(destination.query)), found.why.reason(),
            )
            is ApiResult.Refused -> Resolution.Unrouted(
                Target(destination.query, null, MapsTarget.Query(destination.query)), ManualReason.OFFLINE,
            )
        }
        is Destination.ShortLink -> when (val expanded = routes.expand(destination.url)) {
            is ApiResult.Ok -> if (expanded.value is Destination.ShortLink) {
                Resolution.Unrouted(Target(label, null, MapsTarget.Link(destination.url)), ManualReason.OFFLINE)
            } else {
                resolve(expanded.value, label)
            }
            is ApiResult.Unavailable -> Resolution.Unrouted(Target(label, null, MapsTarget.Link(destination.url)), expanded.why.reason())
            is ApiResult.Refused -> if (expanded.code == "no_destination") {
                Resolution.Failed(InputError.NO_DESTINATION, "")
            } else {
                Resolution.Unrouted(Target(label, null, MapsTarget.Link(destination.url)), ManualReason.OFFLINE)
            }
        }
    }

    private suspend fun route(target: Target) {
        val to = target.at ?: return manual(ManualReason.OFFLINE)
        mutableState.update { it.copy(step = ResolvingStep.FINDING_LOCATION) }
        val from = startCoordinate() ?: return manual(ManualReason.NO_LOCATION)
        mutableState.update { it.copy(step = ResolvingStep.ROUTING) }
        when (val trip = routes.route(from, to)) {
            is ApiResult.Ok -> showResult(trip.value)
            is ApiResult.Unavailable -> manual(trip.why.reason())
            is ApiResult.Refused -> manual(if (trip.code == "no_route") ManualReason.NO_ROUTE else ManualReason.OFFLINE)
        }
    }

    private suspend fun startCoordinate(): Coordinate? = when (val start = state.value.start) {
        is Start.Chosen -> start.place.at
        Start.MyLocation -> {
            askForLocationOnce()
            location.current()?.also { here = it }
        }
    }

    private fun startCoordinateIfKnown(): Coordinate? = when (val start = state.value.start) {
        is Start.Chosen -> start.place.at
        Start.MyLocation -> here
    }

    /** Asks in context, the first time a trip needs the start (R19), and waits for the answer. */
    private suspend fun askForLocationOnce() {
        if (hasLocationPermission() || permissionAsked) return
        permissionAsked = true
        val answer = CompletableDeferred<Unit>().also { permissionAnswer = it }
        mutableState.update { it.copy(askLocation = true) }
        answer.await()
    }

    private fun manual(reason: ManualReason) {
        val trip = state.value.trip
        mutableState.update {
            it.copy(
                stack = it.stack.dropLastWhile { screen -> screen == Screen.Resolving || screen == Screen.ManualTrip } +
                    Screen.ManualTrip,
                step = null, manualReason = reason,
                manualKm = trip?.let { t -> kilometresInput(t.metres) }.orEmpty(),
                manualMinutes = trip?.let { t -> formatMinutes(t.seconds) }.orEmpty(),
                manualKmInvalid = false, manualMinutesInvalid = false,
            )
        }
    }

    fun onManualKm(text: String) = mutableState.update { it.copy(manualKm = text, manualKmInvalid = false) }

    fun onManualMinutes(text: String) = mutableState.update { it.copy(manualMinutes = text, manualMinutesInvalid = false) }

    fun submitManual() {
        val metres = parseKilometres(state.value.manualKm)
        val seconds = parseMinutes(state.value.manualMinutes)
        if (metres == null || seconds == null) {
            return mutableState.update { it.copy(manualKmInvalid = metres == null, manualMinutesInvalid = seconds == null) }
        }
        showResult(Trip(metres, seconds))
    }

    /** Manual entry is always available (§8), prefilled with the current trip. */
    fun editTrip() = manual(ManualReason.EDIT)

    private fun showResult(trip: Trip) {
        mutableState.update { it.copy(trip = trip, step = null, stack = listOf(Screen.Home, Screen.Result)) }
        reprice()
    }

    // --- Cars near you (§4.1) ----------------------------------------------------------

    fun addCars() = push(Screen.Providers)

    fun pickProvider(providerId: String) {
        val draft = state.value.choices.forProvider(providerId, offerIds(providerId))
        mutableState.update { it.copy(draft = draft, carFilter = "") }
        push(Screen.Cars(providerId))
    }

    fun toggle(choice: Choice) = mutableState.update { it.copy(draft = toggled(it.draft, choice)) }

    fun onCarFilter(text: String) = mutableState.update { it.copy(carFilter = text) }

    fun commitCars() {
        val providerId = (state.value.screen as? Screen.Cars)?.providerId ?: return
        val choices = store.load().replacingProvider(providerId, offerIds(providerId), state.value.draft)
        store.save(choices)
        mutableState.update { it.copy(choices = choices, draft = emptySet(), selected = null, stack = listOf(Screen.Home, Screen.Result)) }
        reprice()
    }

    fun remove(choice: Choice) {
        val choices = store.load() - choice
        store.save(choices)
        mutableState.update { it.copy(choices = choices, selected = it.selected.takeIf { s -> s != choice }) }
        reprice()
    }

    fun select(choice: Choice) = mutableState.update { it.copy(selected = choice) }

    /** Open <provider> for the selected option, else the cheapest; uses its once discounts (R5). */
    fun openChosen() {
        val ranked = state.value.options?.ranked.orEmpty()
        val chosen = ranked.firstOrNull { it.choice == state.value.selected } ?: ranked.firstOrNull() ?: return
        val provider = providerOf(chosen.quote.providerId) ?: return
        val (next, used) = state.value.personal.useOnce(provider.id, today())
        if (used.isNotEmpty()) {
            savePersonal(next)
            mutableState.update { it.copy(usedOnce = used) }
        }
        launcher.openProvider(state.value.personal.appFor(provider))
        if (crowdEnabled && state.value.priceSource == PriceSource.SERVER) {
            opened = QuestionTarget(chosen.quote.offerId, provider.id, chosen.quote.offerName)
        }
    }

    /**
     * "Check in <provider>" from step 2; the ticks stay (§4.1). They are saved first, because
     * Android may kill this app while the provider's app is in front.
     */
    fun checkIn(providerId: String) {
        val choices = store.load().replacingProvider(providerId, offerIds(providerId), state.value.draft)
        store.save(choices)
        mutableState.update { it.copy(choices = choices) }
        reprice()
        providerOf(providerId)?.let { launcher.openProvider(state.value.personal.appFor(it)) }
    }

    fun openMaps() {
        val target = state.value.target ?: return
        launcher.openUrl(mapsUrl(startCoordinateIfKnown(), target.maps))
    }

    fun openLicences() = push(Screen.Licences)

    fun openLanguageSettings() = launcher.openAppLanguageSettings()

    /** Cars expire after 15 minutes even while the app is in the background. */
    fun onResume() {
        val choices = store.load()
        if (choices != state.value.choices) {
            mutableState.update { it.copy(choices = choices) }
            reprice()
        }
        opened?.let { target ->
            opened = null
            ask(target)
        }
    }

    // --- Personal (§6) -----------------------------------------------------------------

    fun openSettings() = push(Screen.Settings)

    fun openDeals() = push(Screen.Deals)

    fun newDeal() {
        mutableState.update { it.copy(dealForm = DealForm(providerId = it.providers.singleOrNull()?.id)) }
        push(Screen.DealEditor)
    }

    fun editDeal(id: String) {
        val deal = state.value.personal.deals.firstOrNull { it.id == id } ?: return
        mutableState.update { it.copy(dealForm = deal.toForm(Locale.getDefault())) }
        push(Screen.DealEditor)
    }

    /** Every edit of the form; marks are cleared until the next save. */
    fun onDealForm(form: DealForm) = mutableState.update { it.copy(dealForm = form.copy(invalid = emptySet())) }

    fun saveDeal() {
        val form = state.value.dealForm ?: return
        val (deal, invalid) = form.validated(newId)
        if (deal == null) return mutableState.update { it.copy(dealForm = form.copy(invalid = invalid)) }
        savePersonal(state.value.personal.withDeal(deal))
        mutableState.update { it.copy(dealForm = null) }
        pop()
    }

    fun deleteDeal() {
        val id = state.value.dealForm?.editingId ?: return
        savePersonal(state.value.personal.withoutDeal(id))
        mutableState.update { it.copy(dealForm = null) }
        pop()
    }

    fun toggleDeal(id: String) = savePersonal(state.value.personal.toggled(id))

    fun setRefuel(on: Boolean) {
        mutableState.update { it.copy(refuel = on) }
        reprice()
    }

    fun chooseApp(providerId: String, androidPackage: String) =
        savePersonal(state.value.personal.let { it.copy(apps = it.apps + (providerId to androidPackage)) })

    fun undoOnce() {
        savePersonal(state.value.personal.restore(state.value.usedOnce))
        mutableState.update { it.copy(usedOnce = emptyList()) }
    }

    fun onceMessageShown() = mutableState.update { it.copy(usedOnce = emptyList()) }

    private fun savePersonal(next: Personal) {
        personal.save(next)
        mutableState.update { it.copy(personal = next) }
        reprice()
    }

    // --- Crowd (§7, §5.1) --------------------------------------------------------------

    /** "Price wrong? Tell us": the price form for the option on screen (R9). */
    fun reportPrice() {
        if (!state.value.canReport()) return
        val quote = state.value.reportTarget() ?: return
        val purpose = PricePurpose.Report(quote.offerId, quote.providerId, quote.offerName)
        mutableState.update { it.copy(priceForm = priceForm(purpose, currentCards(quote.offerId), Locale.getDefault())) }
        push(Screen.PriceForm)
    }

    /** "Car not listed? Add it" (§5.1, R14). */
    fun proposeCar(providerId: String) {
        mutableState.update { it.copy(priceForm = PriceForm(PricePurpose.Propose(providerId))) }
        push(Screen.PriceForm)
    }

    fun onPriceForm(form: PriceForm) = mutableState.update { it.copy(priceForm = form.copy(invalid = emptySet())) }

    fun savePrice() {
        val form = state.value.priceForm ?: return
        val question = state.value.question?.takeIf { it.step == QuestionStep.FIX }
        val current = question?.question?.cards
            ?: (form.purpose as? PricePurpose.Report)?.let { currentCards(it.offerId) }
            ?: emptyList()
        val template = (form.purpose as? PricePurpose.Propose)?.let { template(it.providerId) }
        when (val built = form.build(current, template)) {
            is Built.Invalid -> mutableState.update { it.copy(priceForm = form.copy(invalid = built.fields)) }
            Built.Unchanged -> if (question != null) dontKnow() else closeForm()
            is Built.Report -> {
                val offerId = (form.purpose as PricePurpose.Report).offerId
                savePersonal(state.value.personal.withOverride(offerId, built.cards))
                if (question != null) {
                    mutableState.update { it.copy(priceForm = null, question = question.copy(step = QuestionStep.CORRECTED)) }
                } else {
                    closeForm()
                }
                send(built.body, offerId, quiet = question != null)
            }
            is Built.Proposal -> {
                closeForm()
                send(built.body, offerId = null, quiet = false)
            }
        }
    }

    /** "Use the shared price" (R10). */
    fun resetPrice(offerId: String) = savePersonal(state.value.personal.withoutOverride(offerId))

    fun answerYes() {
        val asked = state.value.question ?: return
        mutableState.update { it.copy(question = asked.copy(step = QuestionStep.CONFIRMED)) }
        viewModelScope.launch {
            val reportId = asked.question.reportId
            val result = if (reportId != null) crowd.vote(reportId, Answer.YES) else crowd.confirm(asked.question.offerId, Answer.YES)
            if (result is ApiResult.Ok) refreshPrices()
            else mutableState.update { it.copy(question = it.question?.copy(step = QuestionStep.NOT_SENT)) }
        }
    }

    /** "No, fix it"; for a proposal, a plain no (R13). */
    fun answerNo() {
        val asked = state.value.question ?: return
        if (asked.isProposal) return dontKnow()
        val purpose = PricePurpose.Report(asked.question.offerId, asked.providerId, asked.offerName)
        mutableState.update {
            it.copy(question = asked.copy(step = QuestionStep.FIX), priceForm = priceForm(purpose, asked.question.cards, Locale.getDefault()))
        }
    }

    /** "I don't know the numbers": a no on a pending report, nothing on a current price (decision 36). */
    fun dontKnow() {
        val asked = state.value.question ?: return
        asked.question.reportId?.let { report -> viewModelScope.launch { crowd.vote(report, Answer.NO) } }
        closeQuestion()
    }

    /** Skip and Close. */
    fun closeQuestion() = mutableState.update { it.copy(question = null, priceForm = null) }

    fun crowdMessageShown() = mutableState.update { it.copy(crowdMessage = null) }

    private fun ask(target: QuestionTarget) {
        val prices = prices ?: return
        val nowSeconds = now() / 1_000
        val question = prices.question(target.offerId, nowSeconds, state.value.personal.lastAskedDay) ?: return
        savePersonal(state.value.personal.copy(lastAskedDay = dayOf(nowSeconds)))
        mutableState.update { it.copy(question = QuestionState(question, target.providerId, target.offerName)) }
    }

    private fun send(body: String, offerId: String?, quiet: Boolean) {
        viewModelScope.launch {
            val result = crowd.report(body)
            val message = when {
                result is ApiResult.Ok && result.value.status == "current" -> {
                    offerId?.let { savePersonal(state.value.personal.withoutOverride(it)) }
                    CrowdMessage.ALREADY_CURRENT
                }
                result is ApiResult.Ok -> if (offerId == null) CrowdMessage.PROPOSED else CrowdMessage.SENT
                result is ApiResult.Refused && result.code == "offer_locked" -> CrowdMessage.LOCKED
                else -> CrowdMessage.NOT_SENT
            }
            if (!(quiet && message == CrowdMessage.SENT)) mutableState.update { it.copy(crowdMessage = message) }
            if (result is ApiResult.Ok) refreshPrices()
        }
    }

    private fun refreshPrices() {
        viewModelScope.launch {
            val fresh = priceLoader.load()
            if (fresh.source == PriceSource.SERVER) usePrices(fresh.prices to fresh.source)
        }
    }

    private fun closeForm() {
        mutableState.update { it.copy(priceForm = null) }
        if (state.value.screen == Screen.PriceForm) pop()
    }

    /** The cards the user sees for an offer: their own, else the shared ones. */
    private fun currentCards(offerId: String) =
        state.value.personal.overrideCards(offerId) ?: prices?.cards(offerId).orEmpty()

    /** The provider's first standard card: a proposal copies its label and rounding (R14). */
    private fun template(providerId: String): RateCard? {
        val prices = prices ?: return null
        return state.value.providers.firstOrNull { it.id == providerId }?.offers
            ?.firstNotNullOfOrNull { offer -> prices.cards(offer.id).firstOrNull { it.kind == CardKind.STANDARD } }
    }

    // --- Navigation --------------------------------------------------------------------

    /** System back; false when there is nothing left to go back to (the activity finishes). */
    fun back(): Boolean {
        val stack = state.value.stack
        if (stack.size <= 1) return false
        if (stack.last() == Screen.Resolving) {
            tripJob?.cancel()
            permissionAnswer?.cancel()
        }
        mutableState.update {
            it.copy(
                stack = stack.dropLast(1), step = null, askLocation = false,
                dealForm = if (stack.last() == Screen.DealEditor) null else it.dealForm,
                priceForm = if (stack.last() == Screen.PriceForm) null else it.priceForm,
            )
        }
        return true
    }

    private fun push(screen: Screen) = mutableState.update { it.copy(stack = it.stack + screen) }

    private fun pop() = mutableState.update { it.copy(stack = it.stack.dropLast(1).ifEmpty { listOf(Screen.Home) }) }

    // --- Prices ------------------------------------------------------------------------

    private fun usePrices(loaded: Pair<Prices, PriceSource>) {
        prices = loaded.first
        mutableState.update { it.copy(priceSource = loaded.second, providers = loaded.first.providers()) }
        if (loaded.second == PriceSource.SERVER) {
            val caughtUp = state.value.personal.droppingOverridesEqualTo { loaded.first.cards(it) }
            if (caughtUp != state.value.personal) {
                personal.save(caughtUp)
                mutableState.update { it.copy(personal = caughtUp) }
            }
        }
        val offers = loaded.first.providers().flatMap { p -> p.offers.map { it.id } }
        mutableState.update { it.copy(lockedOffers = offers.filterTo(mutableSetOf()) { id -> loaded.first.isLocked(id) }) }
        reprice()
    }

    private fun reprice() {
        val prices = prices
        val trip = state.value.trip
        if (prices == null || trip == null) return mutableState.update { it.copy(quotes = emptyList(), options = null) }
        val nowSeconds = now() / 1_000
        val adjustments = state.value.personal.adjustments(today(), state.value.refuel)
        val options = prices.options(trip, state.value.choices.toList(), adjustments, nowSeconds)
        mutableState.update {
            it.copy(
                quotes = prices.quotes(trip, adjustments, nowSeconds),
                options = options,
                today = today(),
                selected = it.selected?.takeIf { s -> options.ranked.any { o -> o.choice == s } },
            )
        }
    }

    /** The provider's offers and its quoted proposals, so proposals belong to its choices (R14). */
    private fun offerIds(providerId: String): Set<String> =
        state.value.providers.firstOrNull { it.id == providerId }?.offers?.mapTo(mutableSetOf()) { it.id }.orEmpty() +
            state.value.quotes.filter { it.providerId == providerId }.map { it.offerId }

    private fun providerOf(providerId: String) = state.value.providers.firstOrNull { it.id == providerId }

    private companion object {
        const val SEARCH_DELAY_MILLIS = 300L
        const val MIN_SEARCH_CHARS = 3
        const val MAX_QUERY_CHARS = 200
    }
}

private fun Why.reason() = if (this == Why.BUSY) ManualReason.BUSY else ManualReason.OFFLINE

private fun LinkException.toInputError() = when (this) {
    is LinkException.TooLong -> InputError.TOO_LONG
    is LinkException.NoDestination, is LinkException.InvalidCoordinates -> InputError.NO_DESTINATION
    else -> InputError.NOT_A_MAPS_LINK
}

