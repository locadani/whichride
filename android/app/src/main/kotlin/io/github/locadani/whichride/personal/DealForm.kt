package io.github.locadani.whichride.personal

import io.github.locadani.whichride.format.eurosInput
import io.github.locadani.whichride.format.parseEuros
import io.github.locadani.whichride.format.parsePercent
import io.github.locadani.whichride.format.percentInput
import java.util.Locale

enum class DealKind { PERCENT, AMOUNT, PASS, REFUEL }

enum class DealField { PROVIDER, AMOUNT, TIME_OFF, PACKAGE_OFF }

/**
 * The canvas's "New discount" form. `amount` is the percentage (percent, and a pass's per-km
 * saving) or euros (amount, refuel credit). `enabled` survives editing.
 */
data class DealForm(
    val editingId: String? = null,
    val providerId: String? = null,
    val kind: DealKind = DealKind.PERCENT,
    val amount: String = "",
    val timeOff: String = "",
    val packageOff: String = "",
    val waiveUnlock: Boolean = false,
    val once: Boolean = false,
    val until: Long? = null,
    val enabled: Boolean = true,
    val invalid: Set<DealField> = emptySet(),
)

/** The deal this form describes, or the fields to fix. */
fun DealForm.validated(newId: () -> String): Pair<Deal?, Set<DealField>> {
    val invalid = mutableSetOf<DealField>()
    if (providerId == null) invalid += DealField.PROVIDER
    val id = editingId ?: newId()
    val provider = providerId.orEmpty()
    val deal: Deal? = when (kind) {
        DealKind.PERCENT -> parsePercent(amount)?.let { PercentOff(id, provider, it, until, enabled) }
            .also { if (it == null) invalid += DealField.AMOUNT }
        DealKind.AMOUNT -> parseEuros(amount)?.takeIf { it > 0 }?.let { AmountOff(id, provider, it, once, until, enabled = enabled) }
            .also { if (it == null) invalid += DealField.AMOUNT }
        DealKind.REFUEL -> parseEuros(amount)?.takeIf { it > 0 }?.let { RefuelCreditDeal(id, provider, it, enabled) }
            .also { if (it == null) invalid += DealField.AMOUNT }
        DealKind.PASS -> {
            fun optional(text: String, field: DealField): Int? =
                if (text.isBlank()) 0 else parsePercent(text) ?: run { invalid += field; null }
            val km = optional(amount, DealField.AMOUNT)
            val minutes = optional(timeOff, DealField.TIME_OFF)
            val packages = optional(packageOff, DealField.PACKAGE_OFF)
            if (km == 0 && minutes == 0 && packages == 0 && !waiveUnlock) invalid += DealField.AMOUNT
            if (km != null && minutes != null && packages != null) Pass(id, provider, km, minutes, packages, waiveUnlock, enabled) else null
        }
    }
    return if (invalid.isEmpty()) deal to invalid else null to invalid
}

fun Deal.toForm(locale: Locale): DealForm = when (this) {
    is PercentOff -> DealForm(id, providerId, DealKind.PERCENT, percentInput(basisPoints, locale), until = until, enabled = enabled)
    is AmountOff -> DealForm(id, providerId, DealKind.AMOUNT, eurosInput(cents, locale), once = once, until = until, enabled = enabled)
    is RefuelCreditDeal -> DealForm(id, providerId, DealKind.REFUEL, eurosInput(cents, locale), enabled = enabled)
    is Pass -> DealForm(
        id, providerId, DealKind.PASS,
        amount = if (distanceOff == 0) "" else percentInput(distanceOff, locale),
        timeOff = if (timeOff == 0) "" else percentInput(timeOff, locale),
        packageOff = if (packageOff == 0) "" else percentInput(packageOff, locale),
        waiveUnlock = waiveUnlock, enabled = enabled,
    )
}
