package io.github.locadani.whichride.location

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationManager
import android.os.Build
import androidx.core.content.ContextCompat
import androidx.core.location.LocationManagerCompat
import androidx.core.os.CancellationSignal
import io.github.locadani.whichride.core.Coordinate
import kotlin.coroutines.resume
import kotlin.math.roundToInt
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull

/** Where the phone is now; null without permission or when no fix comes within 15 s (R19). */
fun interface LocationSource {
    suspend fun current(): Coordinate?
}

fun hasLocationPermission(context: Context): Boolean = listOf(
    Manifest.permission.ACCESS_FINE_LOCATION,
    Manifest.permission.ACCESS_COARSE_LOCATION,
).any { ContextCompat.checkSelfPermission(context, it) == PackageManager.PERMISSION_GRANTED }

/** One fresh fix, kept in memory only (§9). */
class AndroidLocationSource(private val context: Context) : LocationSource {
    override suspend fun current(): Coordinate? {
        if (!hasLocationPermission(context)) return null
        val manager = context.getSystemService(LocationManager::class.java) ?: return null
        val provider = listOf(
            LocationManager.FUSED_PROVIDER.takeIf { Build.VERSION.SDK_INT >= Build.VERSION_CODES.S },
            LocationManager.NETWORK_PROVIDER,
            LocationManager.GPS_PROVIDER,
        ).filterNotNull().firstOrNull { manager.isProviderEnabled(it) } ?: return null
        return withTimeoutOrNull(TIMEOUT_MILLIS) {
            suspendCancellableCoroutine { continuation ->
                val signal = CancellationSignal()
                continuation.invokeOnCancellation { signal.cancel() }
                try {
                    LocationManagerCompat.getCurrentLocation(
                        manager, provider, signal, ContextCompat.getMainExecutor(context),
                    ) { location: Location? -> continuation.resume(location?.toCoordinate()) }
                } catch (_: SecurityException) { // permission revoked between the check and the call
                    continuation.resume(null)
                }
            }
        }
    }

    private companion object {
        const val TIMEOUT_MILLIS = 15_000L
    }
}

private fun Location.toCoordinate() = Coordinate((latitude * 1e7).roundToInt(), (longitude * 1e7).roundToInt())
