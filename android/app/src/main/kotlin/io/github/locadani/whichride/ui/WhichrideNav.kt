package io.github.locadani.whichride.ui

import android.Manifest
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.locadani.whichride.compare.CompareViewModel
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.designsystem.WhichrideTheme

/** Shows the top of the ViewModel's back stack (decision 63) and asks for location when it says so. */
@Composable
fun WhichrideNav(viewModel: CompareViewModel) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    BackHandler(enabled = state.stack.size > 1) { viewModel.back() }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) {
        viewModel.onLocationAnswered()
    }
    LaunchedEffect(state.askLocation) {
        if (state.askLocation) {
            permission.launch(arrayOf(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION))
        }
    }
    Surface(Modifier.fillMaxSize(), color = WhichrideTheme.colors.background, contentColor = WhichrideTheme.colors.onSurface) {
        Box(Modifier.safeDrawingPadding()) {
            when (val screen = state.screen) {
                Screen.Home -> HomeScreen(
                    state, viewModel::onQuery, viewModel::submitQuery, viewModel::pickSuggestion,
                    viewModel::changeStart, viewModel::openLicences,
                )
                Screen.Resolving -> ResolvingScreen(state, onCancel = { viewModel.back() })
                Screen.ManualTrip -> ManualTripScreen(
                    state, onBack = { viewModel.back() }, viewModel::onManualKm, viewModel::onManualMinutes,
                    viewModel::submitManual, viewModel::openMaps, viewModel::changeStart,
                )
                Screen.Result -> ResultScreen(
                    state, onBack = { viewModel.back() }, viewModel::editTrip, viewModel::addCars, viewModel::remove,
                    viewModel::select, viewModel::openChosen, viewModel::openMaps,
                )
                Screen.Providers -> ProvidersScreen(state, onClose = { viewModel.back() }, viewModel::pickProvider)
                is Screen.Cars -> CarsScreen(
                    state, screen.providerId, onBack = { viewModel.back() }, viewModel::checkIn, viewModel::toggle,
                    viewModel::onCarFilter, viewModel::commitCars,
                )
                Screen.Start -> StartScreen(state, onBack = { viewModel.back() }, viewModel::onStartQuery, viewModel::pickStart, viewModel::useMyLocation)
                Screen.Licences -> LicencesScreen(onBack = { viewModel.back() })
            }
        }
    }
}
