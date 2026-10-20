package io.github.locadani.whichride.launch

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri

/** Opens other apps; the screens never build intents themselves. */
interface Launcher {
    /** The provider's app, or its Play Store page when it isn't installed or visible (decision 62). */
    fun openProvider(androidPackage: String)
    /** A Google Maps URL: in the Maps app when installed, else any handler. */
    fun openUrl(url: String)
}

class AndroidLauncher(private val context: Context) : Launcher {
    /**
     * Launcher activities don't accept implicit intents (no `CATEGORY_DEFAULT`), so the app is
     * started through its explicit launch intent; that needs the package in the manifest's
     * `<queries>`. A provider missing there opens its Play Store page, which offers "Open".
     */
    override fun openProvider(androidPackage: String) {
        val app = context.packageManager.getLaunchIntentForPackage(androidPackage)
        (app != null && start(app)) ||
            start(Intent(Intent.ACTION_VIEW, Uri.parse("market://details?id=$androidPackage"))) ||
            start(Intent(Intent.ACTION_VIEW, Uri.parse("https://play.google.com/store/apps/details?id=$androidPackage")))
    }

    override fun openUrl(url: String) {
        val uri = Uri.parse(url)
        start(Intent(Intent.ACTION_VIEW, uri).setPackage(GOOGLE_MAPS)) || start(Intent(Intent.ACTION_VIEW, uri))
    }

    private fun start(intent: Intent): Boolean = try {
        context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        true
    } catch (_: ActivityNotFoundException) {
        false
    }

    private companion object {
        const val GOOGLE_MAPS = "com.google.android.apps.maps"
    }
}
