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
import io.github.locadani.whichride.R
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.Alignment
import androidx.compose.runtime.remember
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.SnackbarHost
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
            viewModel.locationRequestLaunched()
        }
    }
    val snackbar = remember { SnackbarHostState() }
    OnceUndo(state, snackbar, viewModel::undoOnce, viewModel::onceMessageShown)
    RemovedUndo(state, snackbar, viewModel::undoRemove, viewModel::removedMessageShown)
    val crowdText = state.crowdMessage?.let { stringResource(it.text()) }
    LaunchedEffect(state.crowdMessage) {
        if (crowdText == null) return@LaunchedEffect
        snackbar.showSnackbar(crowdText)
        viewModel.crowdMessageShown()
    }
    Surface(Modifier.fillMaxSize(), color = WhichrideTheme.colors.background, contentColor = WhichrideTheme.colors.onSurface) {
        Box(Modifier.safeDrawingPadding()) {
            when (val screen = state.screen) {
                Screen.Home -> HomeScreen(
                    state, viewModel::onQuery, viewModel::submitQuery, viewModel::pickSuggestion,
                    viewModel::changeStart, viewModel::openSettings,
                )
                Screen.Resolving -> ResolvingScreen(state, onCancel = { viewModel.back() })
                Screen.ManualTrip -> ManualTripScreen(
                    state, onBack = { viewModel.back() }, viewModel::onManualKm, viewModel::onManualMinutes,
                    viewModel::submitManual, viewModel::openMaps, viewModel::changeStart,
                )
                Screen.Result -> ResultScreen(
                    state,
                    ResultActions(
                        onBack = { viewModel.back() }, onToggleTrip = viewModel::toggleTrip, onTripKm = viewModel::onTripKm,
                        onTripMinutes = viewModel::onTripMinutes, onResetKm = viewModel::resetTripKm, onResetMinutes = viewModel::resetTripMinutes,
                        onAddCars = viewModel::addCars, onRemove = viewModel::remove, onSelect = viewModel::select,
                        onOpen = viewModel::openChosen, onOpenMaps = viewModel::openMaps, onDeals = viewModel::openDeals,
                        onRefuel = viewModel::setRefuel, onReport = viewModel::reportPrice,
                    ),
                )
                Screen.Providers -> ProvidersScreen(state, onClose = { viewModel.back() }, viewModel::pickProvider)
                is Screen.Cars -> CarsScreen(
                    state, screen.providerId, onBack = { viewModel.back() }, viewModel::checkIn, viewModel::toggle,
                    viewModel::onCarFilter, viewModel::commitCars, onPropose = viewModel::proposeCar,
                )
                Screen.Start -> StartScreen(state, onBack = { viewModel.back() }, viewModel::onStartQuery, viewModel::pickStart, viewModel::useMyLocation)
                Screen.Licences -> LicencesScreen(onBack = { viewModel.back() })
                Screen.PriceForm -> PriceFormScreen(state, onBack = { viewModel.back() }, viewModel::onPriceForm, viewModel::savePrice) {
                    viewModel.resetPrice(it)
                    viewModel.back()
                }
                Screen.Settings -> SettingsScreen(
                    state, onBack = { viewModel.back() }, viewModel::openDeals, viewModel::chooseApp,
                    viewModel::openLanguageSettings, viewModel::openLicences,
                )
                Screen.Deals -> DealsScreen(state, onBack = { viewModel.back() }, viewModel::newDeal, viewModel::editDeal, viewModel::toggleDeal)
                Screen.DealEditor -> DealEditorScreen(
                    state, onBack = { viewModel.back() }, viewModel::onDealForm, viewModel::saveDeal, viewModel::deleteDeal,
                )
            }
            QuestionSheet(
                state,
                QuestionActions(
                    viewModel::answerYes, viewModel::answerNo, viewModel::dontKnow, viewModel::closeQuestion,
                    viewModel::onPriceForm, viewModel::savePrice,
                ),
            )
            SnackbarHost(snackbar, Modifier.align(Alignment.BottomCenter))
        }
    }
}
