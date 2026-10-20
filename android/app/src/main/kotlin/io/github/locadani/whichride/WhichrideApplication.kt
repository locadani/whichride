package io.github.locadani.whichride

import android.app.Application

class WhichrideApplication : Application() {
    val container: AppContainer by lazy { AppContainer(this) }
}
