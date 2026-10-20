package io.github.locadani.whichride.options

import android.content.SharedPreferences
import io.github.locadani.whichride.core.Choice

/**
 * The cars the user added (spec §4.1, R18): offer and provider IDs only, kept on the device and
 * dropped 15 minutes after the last change (cars move). Never sent anywhere.
 */
/** The added cars between launches; the ViewModel's tests fake it. */
interface ChoiceStore {
    fun load(): Set<Choice>
    fun save(choices: Set<Choice>)
}

class OptionsStore(private val prefs: SharedPreferences, private val now: () -> Long) : ChoiceStore {
    override fun load(): Set<Choice> {
        val changedAt = prefs.getLong(CHANGED_AT, Long.MIN_VALUE)
        val age = now() - changedAt
        if (changedAt == Long.MIN_VALUE || age < 0 || age >= LIFETIME_MILLIS) {
            clear()
            return emptySet()
        }
        return prefs.getStringSet(CHOICES, emptySet()).orEmpty().mapNotNullTo(mutableSetOf(), ::decode)
    }

    override fun save(choices: Set<Choice>) {
        if (choices.isEmpty()) return clear()
        prefs.edit().putStringSet(CHOICES, choices.mapTo(mutableSetOf(), ::encode)).putLong(CHANGED_AT, now()).apply()
    }

    private fun clear() = prefs.edit().remove(CHOICES).remove(CHANGED_AT).apply()

    private fun encode(choice: Choice) = when (choice) {
        is Choice.Offer -> "$OFFER${choice.offerId}"
        is Choice.AnyOf -> "$ANY${choice.providerId}"
    }

    private fun decode(text: String): Choice? = when {
        text.startsWith(OFFER) -> Choice.Offer(text.removePrefix(OFFER))
        text.startsWith(ANY) -> Choice.AnyOf(text.removePrefix(ANY))
        else -> null
    }

    companion object {
        const val LIFETIME_MILLIS = 15 * 60_000L
        private const val CHOICES = "choices"
        private const val CHANGED_AT = "changed_at"
        private const val OFFER = "offer:"
        private const val ANY = "any:"
    }
}
