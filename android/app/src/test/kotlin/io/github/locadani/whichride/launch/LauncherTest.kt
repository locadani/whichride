package io.github.locadani.whichride.launch

import android.app.Application
import android.content.ComponentName
import android.content.Intent
import android.content.IntentFilter
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Shadows.shadowOf

@RunWith(AndroidJUnit4::class)
class LauncherTest {
    private val app = ApplicationProvider.getApplicationContext<Application>()
    private val launcher = AndroidLauncher(app)

    @Before fun strict() = shadowOf(app).checkActivities(true)

    private fun install(pkg: String, filter: IntentFilter) {
        val component = ComponentName(pkg, "$pkg.Main")
        shadowOf(app.packageManager).addActivityIfNotPresent(component)
        shadowOf(app.packageManager).addIntentFilterForActivity(component, filter)
    }

    @Test fun opensAnInstalledProviderApp() {
        install("ee.mtakso.client", IntentFilter(Intent.ACTION_MAIN).apply { addCategory(Intent.CATEGORY_LAUNCHER) })
        launcher.openProvider("ee.mtakso.client")
        val started = shadowOf(app).nextStartedActivity
        assertEquals("ee.mtakso.client", started.component?.packageName)
    }

    @Test fun opensThePlayStoreWhenTheAppIsMissing() {
        install("com.android.vending", IntentFilter(Intent.ACTION_VIEW).apply { addDataScheme("market"); addCategory(Intent.CATEGORY_DEFAULT) })
        launcher.openProvider("com.trafi.whitelabel.bvg")
        assertEquals("market://details?id=com.trafi.whitelabel.bvg", shadowOf(app).nextStartedActivity.dataString)
    }

    @Test fun opensTheWebStoreWithoutAStoreApp() {
        install("org.browser", IntentFilter(Intent.ACTION_VIEW).apply { addDataScheme("https"); addCategory(Intent.CATEGORY_DEFAULT) })
        launcher.openProvider("com.driveby.app")
        assertEquals("https://play.google.com/store/apps/details?id=com.driveby.app", shadowOf(app).nextStartedActivity.dataString)
    }

    @Test fun opensMapsUrlsInGoogleMapsFirst() {
        install("com.google.android.apps.maps", IntentFilter(Intent.ACTION_VIEW).apply { addDataScheme("https"); addCategory(Intent.CATEGORY_DEFAULT) })
        launcher.openUrl("https://www.google.com/maps/dir/?api=1&destination=52.5,13.4&travelmode=driving")
        assertEquals("com.google.android.apps.maps", shadowOf(app).nextStartedActivity.`package`)
    }
}
