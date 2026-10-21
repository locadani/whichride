package io.github.locadani.whichride

import android.app.Application
import android.content.Context
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import io.github.locadani.whichride.compare.CompareViewModel
import io.github.locadani.whichride.crowd.ServerCrowd
import io.github.locadani.whichride.launch.AndroidLauncher
import io.github.locadani.whichride.location.AndroidLocationSource
import io.github.locadani.whichride.location.hasLocationPermission
import io.github.locadani.whichride.options.OptionsStore
import io.github.locadani.whichride.personal.FilePersonalStore
import io.github.locadani.whichride.prices.PriceBook
import io.github.locadani.whichride.server.Installs
import io.github.locadani.whichride.server.IntegrityTokens
import io.github.locadani.whichride.server.KeystoreTokenStore
import io.github.locadani.whichride.server.PlayIntegrityTokens
import io.github.locadani.whichride.server.ServerApi
import io.github.locadani.whichride.server.ServerRoutes
import io.github.locadani.whichride.server.chooseIntegrityTokens
import java.io.File
import java.util.concurrent.TimeUnit
import okhttp3.OkHttpClient

/** Wires the app by hand (R33). */
class AppContainer(private val app: Application) {
    private val http = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .callTimeout(20, TimeUnit.SECONDS)
        .build()
    private val api = ServerApi(BuildConfig.SERVER_URL, http)
    private val integrity: IntegrityTokens =
        chooseIntegrityTokens(BuildConfig.REGISTRATION_KEY, BuildConfig.CLOUD_PROJECT_NUMBER) {
            PlayIntegrityTokens(app, BuildConfig.CLOUD_PROJECT_NUMBER)
        }
    private val installs = Installs(api, integrity, KeystoreTokenStore(prefs("install")))
    private val priceBook = PriceBook(api, File(app.filesDir, "catalogue.json")) {
        app.assets.open("seed-catalogue.json").bufferedReader().use { it.readText() }
    }

    val viewModelFactory: ViewModelProvider.Factory = viewModelFactory {
        initializer {
            CompareViewModel(
                routes = ServerRoutes(api, installs),
                location = AndroidLocationSource(app),
                launcher = AndroidLauncher(app),
                store = OptionsStore(prefs("options"), System::currentTimeMillis),
                priceLoader = priceBook,
                hasLocationPermission = { hasLocationPermission(app) },
                personal = FilePersonalStore(File(app.filesDir, "personal.json")),
                crowd = ServerCrowd(api, installs),
                crowdEnabled = BuildConfig.SERVER_URL.isNotEmpty(),
            )
        }
    }

    private fun prefs(name: String) = app.getSharedPreferences(name, Context.MODE_PRIVATE)
}
