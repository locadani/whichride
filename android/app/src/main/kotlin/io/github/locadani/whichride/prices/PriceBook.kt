package io.github.locadani.whichride.prices

import io.github.locadani.whichride.core.DataException
import io.github.locadani.whichride.core.Prices
import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.ServerApi
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

enum class PriceSource { SERVER, CACHED, BUILT_IN }

class LoadedPrices(val prices: Prices, val source: PriceSource)

/**
 * The prices to compare with (decision 60): the server's catalogue, else the last one it sent,
 * else the seed bundled in the APK. The catalogue is public data, so caching it is no privacy
 * concern; it's validated by the core before use.
 */
interface PriceLoader {
    /** The cached copy, else the seed; no network. */
    suspend fun offline(): LoadedPrices
    /** The server's catalogue, else [offline]. */
    suspend fun load(): LoadedPrices
}

class PriceBook(private val api: ServerApi, private val cacheFile: File, private val seed: () -> String) : PriceLoader {
    override suspend fun load(): LoadedPrices {
        val answer = api.catalogue()
        if (answer is ApiResult.Ok) {
            parse(answer.value)?.let { prices ->
                withContext(Dispatchers.IO) { writeAtomically(answer.value) }
                return LoadedPrices(prices, PriceSource.SERVER)
            }
        }
        return offline()
    }

    override suspend fun offline(): LoadedPrices {
        val cached = withContext(Dispatchers.IO) { cacheFile.takeIf(File::exists)?.readText() }
        cached?.let(::parse)?.let { return LoadedPrices(it, PriceSource.CACHED) }
        // The seed is checked by the build's tests; a broken one is a bug worth crashing on.
        return LoadedPrices(Prices.fromSeed(withContext(Dispatchers.IO) { seed() }), PriceSource.BUILT_IN)
    }

    private fun parse(json: String): Prices? = try {
        Prices.fromServerResponse(json)
    } catch (_: DataException) {
        null
    }

    private fun writeAtomically(json: String) {
        val temporary = File(cacheFile.parentFile, "${cacheFile.name}.tmp")
        temporary.writeText(json)
        temporary.renameTo(cacheFile)
    }
}
