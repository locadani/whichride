package io.github.locadani.whichride.compare

/** One entry of the app's back stack (decision 63). */
sealed interface Screen {
    data object Home : Screen
    data object Resolving : Screen
    data object ManualTrip : Screen
    data object Result : Screen
    data object Providers : Screen
    data class Cars(val providerId: String) : Screen
    data object Start : Screen
    data object Licences : Screen
}
