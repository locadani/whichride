package io.github.locadani.whichride.options

import io.github.locadani.whichride.core.Choice

/** The choices that belong to one provider: its offers and its "any car". */
fun Set<Choice>.forProvider(providerId: String, offerIds: Set<String>): Set<Choice> = filterTo(mutableSetOf()) {
    when (it) {
        is Choice.AnyOf -> it.providerId == providerId
        is Choice.Offer -> it.offerId in offerIds
    }
}

/**
 * One tick in step 2 (§4.1), within one provider's draft: "any car" and specific models
 * exclude each other, as in the prototype.
 */
fun toggle(draft: Set<Choice>, choice: Choice): Set<Choice> = when {
    choice in draft -> draft - choice
    choice is Choice.AnyOf -> setOf(choice)
    else -> draft.filterNotTo(mutableSetOf()) { it is Choice.AnyOf } + choice
}

/** "Add N cars": the provider's choices become the draft; other providers are untouched. */
fun Set<Choice>.replacingProvider(providerId: String, offerIds: Set<String>, draft: Set<Choice>): Set<Choice> =
    this - forProvider(providerId, offerIds) + draft
