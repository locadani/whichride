package io.github.locadani.whichride

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import io.github.locadani.whichride.compare.CompareViewModel
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.entry.sharedText
import io.github.locadani.whichride.ui.WhichrideNav

class MainActivity : ComponentActivity() {
    private val viewModel: CompareViewModel by viewModels {
        (application as WhichrideApplication).container.viewModelFactory
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        // A recreated activity (rotation) already handed its share to the ViewModel.
        if (savedInstanceState == null) sharedText(intent)?.let(viewModel::onShared)
        setContent { WhichrideTheme { WhichrideNav(viewModel) } }
    }

    /** A second share while the app is open replaces the trip (R31). */
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        sharedText(intent)?.let(viewModel::onShared)
    }

    override fun onResume() {
        super.onResume()
        viewModel.onResume()
    }
}
