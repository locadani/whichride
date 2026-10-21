package io.github.locadani.whichride.crowd

import io.github.locadani.whichride.core.CardKind
import io.github.locadani.whichride.core.RateCard
import io.github.locadani.whichride.core.ReportException
import io.github.locadani.whichride.core.proposalBody
import io.github.locadani.whichride.core.reportBody
import io.github.locadani.whichride.core.reportLimits
import io.github.locadani.whichride.format.eurosInput
import io.github.locadani.whichride.format.parseEuros
import java.util.Locale

/** Offer IDs of pending proposals (plan R14; the ffi's `PROPOSAL_PREFIX`). */
const val PROPOSAL_PREFIX = "proposal."

enum class PriceField { NAME, UNLOCK, PER_KM, PER_MINUTE, MINIMUM }

sealed interface PricePurpose {
    /** A report on an existing offer (§7.1), also the "No, fix it" step of the question (§7.2). */
    data class Report(val offerId: String, val providerId: String, val offerName: String) : PricePurpose
    /** "Car not listed? Add it" (§5.1). */
    data class Propose(val providerId: String) : PricePurpose
}

/** The canvas's "What does Bolt show?" form: the standard card's four prices, as typed. */
data class PriceForm(
    val purpose: PricePurpose,
    val name: String = "",
    val unlock: String = "",
    val perKm: String = "",
    val perMinute: String = "",
    val minimum: String = "",
    val invalid: Set<PriceField> = emptySet(),
    val sending: Boolean = false,
)

sealed interface Built {
    data class Report(val body: String, val cards: List<RateCard>) : Built
    data class Proposal(val body: String) : Built
    /** The values are what the user already sees: nothing to send or store (R9). */
    data object Unchanged : Built
    data class Invalid(val fields: Set<PriceField>) : Built
}

/** The card a report edits: the standard one, else the first (plan R9). */
fun List<RateCard>.editable(): RateCard? = firstOrNull { it.kind == CardKind.STANDARD } ?: firstOrNull()

fun priceForm(purpose: PricePurpose, cards: List<RateCard>, locale: Locale): PriceForm {
    val card = cards.editable() ?: return PriceForm(purpose)
    return PriceForm(
        purpose,
        unlock = eurosInput(card.baseFee, locale),
        perKm = eurosInput(card.perKm, locale),
        perMinute = eurosInput(card.perMinute, locale),
        minimum = eurosInput(card.minimum, locale),
    )
}

/**
 * The request body and cards this form stands for, checked with the core's limits (decision 65).
 * `current` are the offer's cards as the user sees them; `template` is the provider's first
 * standard card, whose label and rounding a proposal copies.
 */
fun PriceForm.build(current: List<RateCard>, template: RateCard?): Built {
    val limits = reportLimits()
    val invalid = mutableSetOf<PriceField>()
    fun euros(text: String, max: Long, field: PriceField, emptyIsZero: Boolean = false): Long =
        (if (emptyIsZero && text.isBlank()) 0L else parseEuros(text))?.takeIf { it <= max } ?: run { invalid += field; 0L }
    val unlock = euros(unlock, limits.maxBaseFee, PriceField.UNLOCK, emptyIsZero = true)
    val perKm = euros(perKm, limits.maxPerUnit, PriceField.PER_KM, emptyIsZero = true)
    val perMinute = euros(perMinute, limits.maxPerUnit, PriceField.PER_MINUTE, emptyIsZero = true)
    val minimum = euros(minimum, limits.maxMinimum, PriceField.MINIMUM, emptyIsZero = true)
    if (purpose is PricePurpose.Propose && (name.isBlank() || name.trim().length > limits.maxNameChars.toInt())) invalid += PriceField.NAME
    if (invalid.isNotEmpty()) return Built.Invalid(invalid)
    return when (purpose) {
        is PricePurpose.Report -> {
            val base = current.editable() ?: return Built.Invalid(setOf(PriceField.PER_KM))
            val edited = base.copy(baseFee = unlock, perKm = perKm, perMinute = perMinute, minimum = minimum)
            if (edited == base) return Built.Unchanged
            val cards = current.map { if (it === base) edited else it }
            try {
                Built.Report(reportBody(purpose.offerId, cards), cards)
            } catch (_: ReportException) {
                Built.Invalid(setOf(PriceField.PER_KM))
            }
        }
        is PricePurpose.Propose -> {
            val base = template ?: return Built.Invalid(setOf(PriceField.PER_KM))
            val card = base.copy(
                kind = CardKind.STANDARD, baseFee = unlock, perKm = perKm, perMinute = perMinute,
                includedKm = 0u, includedMinutes = 0u, minimum = minimum,
            )
            try {
                Built.Proposal(proposalBody(purpose.providerId, name, listOf(card)))
            } catch (_: ReportException.BadName) {
                Built.Invalid(setOf(PriceField.NAME))
            } catch (_: ReportException) {
                Built.Invalid(setOf(PriceField.PER_KM))
            }
        }
    }
}
