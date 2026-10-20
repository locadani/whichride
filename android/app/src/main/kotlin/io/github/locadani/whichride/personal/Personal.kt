package io.github.locadani.whichride.personal

import io.github.locadani.whichride.core.Adjustments
import io.github.locadani.whichride.core.DataException
import io.github.locadani.whichride.core.Discount
import io.github.locadani.whichride.core.PriceOverride
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.core.RateCard
import io.github.locadani.whichride.core.RefuelCredit
import io.github.locadani.whichride.core.Subscription
import io.github.locadani.whichride.core.cardsFromJson
import io.github.locadani.whichride.core.cardsToJson
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * One of the user's own price changes (spec §6, plan R5–R7). Kept on this phone only. Days are
 * epoch days in the phone's time zone; percentages are basis points.
 */
@Serializable
sealed interface Deal {
    val id: String
    val providerId: String
    val enabled: Boolean
}

@Serializable @SerialName("percent")
data class PercentOff(
    override val id: String,
    override val providerId: String,
    val basisPoints: Int,
    val until: Long?,
    override val enabled: Boolean = true,
) : Deal

@Serializable @SerialName("amount")
data class AmountOff(
    override val id: String,
    override val providerId: String,
    val cents: Long,
    val once: Boolean,
    val until: Long?,
    val used: Boolean = false,
    override val enabled: Boolean = true,
) : Deal

/** A subscription: its monthly fee is not counted per trip (§6). */
@Serializable @SerialName("pass")
data class Pass(
    override val id: String,
    override val providerId: String,
    val distanceOff: Int,
    val timeOff: Int,
    val packageOff: Int,
    val waiveUnlock: Boolean,
    override val enabled: Boolean = true,
) : Deal

/** What the provider credits this user for refuelling or charging (decision 47). */
@Serializable @SerialName("refuel")
data class RefuelCreditDeal(
    override val id: String,
    override val providerId: String,
    val cents: Long,
    override val enabled: Boolean = true,
) : Deal

/** The user's own rate cards for an offer, as the core's JSON (decision 9). */
@Serializable
data class StoredOverride(val offerId: String, val cardsJson: String)

@Serializable
data class Personal(
    val deals: List<Deal> = emptyList(),
    val overrides: List<StoredOverride> = emptyList(),
    /** Provider ID → the Android package the user chose to open it with (decision 66). */
    val apps: Map<String, String> = emptyMap(),
    /** `dayOf` of the last §7.2 question shown (UTC day, as the server counts). */
    val lastAskedDay: Int? = null,
)

/** Enabled, not past its last day, and not a once discount already used. */
fun Deal.isActive(today: Long): Boolean = enabled && when (this) {
    is PercentOff -> until == null || today <= until
    is AmountOff -> !used && (until == null || today <= until)
    is Pass, is RefuelCreditDeal -> true
}

/** What applies to a trip today; `refuel` is the result screen's per-trip switch (R6). */
fun Personal.adjustments(today: Long, refuel: Boolean): Adjustments {
    val active = deals.filter { it.isActive(today) }
    return Adjustments(
        overrides = overrides.mapNotNull { stored -> overrideCards(stored.offerId)?.let { PriceOverride(stored.offerId, it) } },
        subscriptions = active.filterIsInstance<Pass>().map {
            Subscription(it.providerId, it.distanceOff.toUShort(), it.timeOff.toUShort(), it.waiveUnlock, it.packageOff.toUShort())
        },
        discounts = active.mapNotNull {
            when (it) {
                is PercentOff -> Discount.Percent(it.providerId, it.basisPoints.toUShort())
                is AmountOff -> Discount.Amount(it.providerId, it.cents)
                else -> null
            }
        },
        refuelCredits = active.filterIsInstance<RefuelCreditDeal>().map { RefuelCredit(it.providerId, it.cents) },
        refuel = refuel,
    )
}

fun Personal.hasActiveRefuelCredit(today: Long): Boolean = deals.any { it is RefuelCreditDeal && it.isActive(today) }

/** Marks the provider's active once discounts used (plan R5); returns their IDs for Undo. */
fun Personal.useOnce(providerId: String, today: Long): Pair<Personal, List<String>> {
    val used = deals.filter { it is AmountOff && it.once && it.providerId == providerId && it.isActive(today) }.map { it.id }
    if (used.isEmpty()) return this to used
    return copy(deals = deals.map { if (it.id in used) (it as AmountOff).copy(used = true) else it }) to used
}

fun Personal.restore(ids: List<String>): Personal =
    copy(deals = deals.map { if (it.id in ids && it is AmountOff) it.copy(used = false) else it })

fun Personal.withDeal(deal: Deal): Personal =
    if (deals.any { it.id == deal.id }) copy(deals = deals.map { if (it.id == deal.id) deal else it })
    else copy(deals = deals + deal)

fun Personal.withoutDeal(id: String): Personal = copy(deals = deals.filterNot { it.id == id })

fun Personal.toggled(id: String): Personal = copy(
    deals = deals.map {
        if (it.id != id) it
        else when (it) {
            is PercentOff -> it.copy(enabled = !it.enabled)
            is AmountOff -> it.copy(enabled = !it.enabled)
            is Pass -> it.copy(enabled = !it.enabled)
            is RefuelCreditDeal -> it.copy(enabled = !it.enabled)
        }
    },
)

fun Personal.withOverride(offerId: String, cards: List<RateCard>): Personal =
    copy(overrides = overrides.filterNot { it.offerId == offerId } + StoredOverride(offerId, cardsToJson(cards)))

fun Personal.withoutOverride(offerId: String): Personal = copy(overrides = overrides.filterNot { it.offerId == offerId })

/** The user's cards for an offer; null when there are none or they can't be read. */
fun Personal.overrideCards(offerId: String): List<RateCard>? {
    val stored = overrides.firstOrNull { it.offerId == offerId } ?: return null
    return try {
        cardsFromJson(stored.cardsJson)
    } catch (_: DataException) {
        null
    }
}

/** Overrides the crowd has caught up with are no longer needed (plan R10). */
fun Personal.droppingOverridesEqualTo(shared: (offerId: String) -> List<RateCard>): Personal =
    copy(overrides = overrides.filterNot { overrideCards(it.offerId) == shared(it.offerId) })

/** The package that opens this provider: the user's choice while the catalogue still offers it. */
fun Personal.appFor(provider: ProviderInfo): String =
    apps[provider.id]?.takeIf { chosen -> provider.apps.any { it.androidPackage == chosen } } ?: provider.androidPackage
