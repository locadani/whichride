package io.github.locadani.whichride.entry

import android.content.Intent
import android.net.Uri
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SharedTextTest {
    @Test
    fun aShareCarriesItsText() {
        val intent = Intent(Intent.ACTION_SEND).setType("text/plain")
            .putExtra(Intent.EXTRA_TEXT, "Markthalle Neun\nhttps://maps.app.goo.gl/x7Qa2Lm")
        assertEquals("Markthalle Neun\nhttps://maps.app.goo.gl/x7Qa2Lm", sharedText(intent))
    }

    @Test
    fun aGeoLinkIsItsUri() {
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse("geo:52.52,13.405?q=Alexanderplatz"))
        assertEquals("geo:52.52,13.405?q=Alexanderplatz", sharedText(intent))
    }

    @Test
    fun launchesAndBlankSharesCarryNothing() {
        assertNull(sharedText(Intent(Intent.ACTION_MAIN)))
        assertNull(sharedText(Intent(Intent.ACTION_SEND).putExtra(Intent.EXTRA_TEXT, "  ")))
        assertNull(sharedText(null))
    }

    @Test
    fun reopeningFromRecentsDoesNotRepeatTheShare() {
        val intent = Intent(Intent.ACTION_SEND).putExtra(Intent.EXTRA_TEXT, "https://maps.app.goo.gl/x")
            .addFlags(Intent.FLAG_ACTIVITY_LAUNCHED_FROM_HISTORY)
        assertNull(sharedText(intent))
    }
}
