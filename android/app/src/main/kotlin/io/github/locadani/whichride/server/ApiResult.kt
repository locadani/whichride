package io.github.locadani.whichride.server

/** A server call's outcome, already sorted by what the app does next (R25). */
sealed interface ApiResult<out T> {
    data class Ok<T>(val value: T) : ApiResult<T>
    /** The service can't answer now; the comparison continues with typed km and minutes. */
    data class Unavailable(val why: Why) : ApiResult<Nothing>
    /** The server refused this request (`code` from crates/server/README.md). */
    data class Refused(val status: Int, val code: String) : ApiResult<Nothing>
}

enum class Why {
    /** No server URL in this build (R9). */
    NOT_CONFIGURED,
    /** No connection, or the server didn't answer. */
    OFFLINE,
    /** 429, 503 or another 5xx: quota used up, rate limited or down. */
    BUSY,
    /** Play Integrity unavailable or refused: this install can't register (R9). */
    NOT_REGISTERED,
}

inline fun <T, R> ApiResult<T>.map(transform: (T) -> R): ApiResult<R> = when (this) {
    is ApiResult.Ok -> ApiResult.Ok(transform(value))
    is ApiResult.Unavailable -> this
    is ApiResult.Refused -> this
}
