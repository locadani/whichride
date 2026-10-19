package dev.locadani.spike

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import uniffi.spike_core.RateCard
import uniffi.spike_core.tripCostCents

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // 10.2 km, 25 min: computed by the Rust core.
        val miles = tripCostCents(RateCard(100u, 79u, 0u, 0u), 10_200u, 25u)
        val bolt = tripCostCents(RateCard(0u, 18u, 11u, 250u), 10_200u, 25u)
        setContent {
            MaterialTheme {
                Surface {
                    Column(Modifier.padding(24.dp)) {
                        Text("Computed in Rust", style = MaterialTheme.typography.headlineSmall)
                        Text("Example A: €%.2f".format(miles.toInt() / 100.0))
                        Text("Example B: €%.2f".format(bolt.toInt() / 100.0))
                    }
                }
            }
        }
    }
}
