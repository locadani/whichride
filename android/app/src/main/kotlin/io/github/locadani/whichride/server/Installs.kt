package io.github.locadani.whichride.server

import java.security.SecureRandom
import java.util.Base64
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.Json

/**
 * This install's identity on the server (decisions 32–33, 44): registers once with a Play
 * Integrity token bound to a fresh nonce, keeps the bearer token, and registers again once if
 * the server no longer knows it.
 */
class Installs(
    private val api: ServerApi,
    private val integrity: IntegrityTokens,
    private val store: TokenStore,
    private val nonce: () -> String = ::randomNonce,
) {
    private val mutex = Mutex()

    suspend fun <T> authorized(call: suspend (bearer: String) -> ApiResult<T>): ApiResult<T> {
        val first = when (val bearer = bearer()) {
            is ApiResult.Ok -> bearer.value
            is ApiResult.Unavailable -> return bearer
            is ApiResult.Refused -> return bearer
        }
        val result = call(first)
        if (result !is ApiResult.Refused || result.status != 401) return result
        mutex.withLock { if (store.load() == first) store.clear() }
        return when (val again = bearer()) {
            is ApiResult.Ok -> call(again.value)
            is ApiResult.Unavailable -> again
            is ApiResult.Refused -> again
        }
    }

    private suspend fun bearer(): ApiResult<String> = mutex.withLock {
        store.load()?.let { return ApiResult.Ok(it) }
        val body = Json.encodeToString(NonceBody(nonce()))
        val token = integrity.token(registrationHash(body)) ?: return ApiResult.Unavailable(Why.NOT_REGISTERED)
        when (val registered = api.register(body, token)) {
            is ApiResult.Ok -> registered.also { store.save(it.value) }
            is ApiResult.Refused -> ApiResult.Unavailable(Why.NOT_REGISTERED)
            is ApiResult.Unavailable -> registered
        }
    }
}

/** 24 random bytes as 32 base64url characters (the server accepts 16–128 of `[A-Za-z0-9_-]`). */
fun randomNonce(): String =
    Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(24).also(SecureRandom()::nextBytes))
