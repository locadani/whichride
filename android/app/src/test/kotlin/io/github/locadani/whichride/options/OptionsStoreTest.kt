package io.github.locadani.whichride.options

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.core.Choice
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class OptionsStoreTest {
    private var now = 1_000_000L
    private val prefs = ApplicationProvider.getApplicationContext<Context>()
        .getSharedPreferences("options-test", Context.MODE_PRIVATE)
    private val store = OptionsStore(prefs) { now }
    private val choices = setOf(Choice.Offer("bolt.vw-polo"), Choice.AnyOf("miles"))

    @Test fun keepsTheCarsForFifteenMinutes() {
        store.save(choices)
        now += 15 * 60_000L - 1
        assertEquals(choices, store.load())
    }

    @Test fun optionsExpire15MinutesAfterTheLastChange() {
        store.save(choices)
        now += 15 * 60_000L
        assertEquals(emptySet<Choice>(), store.load())
        assertEquals(emptySet<Choice>(), OptionsStore(prefs) { now - 60_000L }.load())
    }

    @Test fun aWrongClockGoingBackwardsExpiresThem() {
        store.save(choices)
        now -= 1
        assertEquals(emptySet<Choice>(), store.load())
    }

    @Test fun survivesANewStoreInstance() {
        store.save(choices)
        assertEquals(choices, OptionsStore(prefs) { now }.load())
    }

    @Test fun unreadableEntriesAreDropped() {
        prefs.edit().putStringSet("choices", setOf("offer:bolt.vw-polo", "garbage")).putLong("changed_at", now).commit()
        assertEquals(setOf(Choice.Offer("bolt.vw-polo")), store.load())
    }
}
