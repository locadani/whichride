package io.github.locadani.whichride.server

import android.content.Context
import com.google.android.play.core.integrity.IntegrityManagerFactory
import com.google.android.play.core.integrity.StandardIntegrityManager.PrepareIntegrityTokenRequest
import com.google.android.play.core.integrity.StandardIntegrityManager.StandardIntegrityTokenProvider
import com.google.android.play.core.integrity.StandardIntegrityManager.StandardIntegrityTokenRequest
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.tasks.await

/** A Play Integrity token bound to `requestHash`, or null when Play can't give one. */
fun interface IntegrityTokens {
    suspend fun token(requestHash: String): String?
}

/** The owner's registration key for every request, before the app is on Play (spec 2026-10-21 §3.2). */
class RegistrationKeyTokens(private val key: String) : IntegrityTokens {
    override suspend fun token(requestHash: String): String = key
}

/** Build-time choice: the registration key if this build has one, else Play when a Cloud project is set. */
fun chooseIntegrityTokens(registrationKey: String, cloudProjectNumber: Long, play: () -> IntegrityTokens): IntegrityTokens =
    when {
        registrationKey.isNotEmpty() -> RegistrationKeyTokens(registrationKey)
        cloudProjectNumber == 0L -> IntegrityTokens { null }
        else -> play()
    }

/** Play Integrity standard requests (decision 33); the provider is prepared once and re-prepared if it expired. */
class PlayIntegrityTokens(context: Context, private val cloudProjectNumber: Long) : IntegrityTokens {
    private val manager = IntegrityManagerFactory.createStandard(context.applicationContext)
    private val mutex = Mutex()
    private var provider: StandardIntegrityTokenProvider? = null

    override suspend fun token(requestHash: String): String? =
        request(requestHash) ?: run { mutex.withLock { provider = null }; request(requestHash) }

    private suspend fun request(requestHash: String): String? = runCatching {
        val provider = mutex.withLock {
            provider ?: manager.prepareIntegrityToken(
                PrepareIntegrityTokenRequest.builder().setCloudProjectNumber(cloudProjectNumber).build(),
            ).await().also { provider = it }
        }
        provider.request(StandardIntegrityTokenRequest.builder().setRequestHash(requestHash).build()).await().token()
    }.onFailure { if (it is CancellationException) throw it }.getOrNull()
}
