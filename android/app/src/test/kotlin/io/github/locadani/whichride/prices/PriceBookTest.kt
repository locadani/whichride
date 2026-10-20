package io.github.locadani.whichride.prices

import io.github.locadani.whichride.server.ServerApi
import java.io.File
import kotlin.io.path.createTempDirectory
import kotlinx.coroutines.test.runTest
import mockwebserver3.MockResponse
import mockwebserver3.MockWebServer
import okhttp3.OkHttpClient
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class PriceBookTest {
    private val seed = File("../../crates/server/catalogue.json").readText()
    private val server = MockWebServer()
    private val cache = createTempDirectory().resolve("catalogue.json").toFile()
    private lateinit var api: ServerApi

    private val response =
        """{"catalogue":$seed,"offers":[{"offer":"miles.s","confirmed_on":20744,"locked":false,"pending":null}],"proposals":[]}"""

    @Before fun start() {
        server.start()
        api = ServerApi(server.url("/").toString().trimEnd('/'), OkHttpClient())
    }

    @After fun stop() = server.close()

    private fun book(api: ServerApi = this.api) = PriceBook(api, cache) { seed }

    @Test fun usesTheServersPricesAndKeepsACopy() = runTest {
        server.enqueue(MockResponse.Builder().body(response).build())
        val loaded = book().load()
        assertEquals(PriceSource.SERVER, loaded.source)
        assertEquals(response, cache.readText())
    }

    @Test fun fallsBackToTheCopyWhenTheServerIsBusy() = runTest {
        cache.writeText(response)
        server.enqueue(MockResponse.Builder().code(503).body("""{"error":"internal"}""").build())
        assertEquals(PriceSource.CACHED, book().load().source)
    }

    @Test fun fallsBackToTheSeedWithoutServerOrCopy() = runTest {
        val loaded = book(ServerApi("", OkHttpClient())).load()
        assertEquals(PriceSource.BUILT_IN, loaded.source)
        assertEquals(3, loaded.prices.providers().size)
        assertFalse(cache.exists())
    }

    @Test fun offlineNeverAsksTheServer() = runTest {
        cache.writeText(response)
        assertEquals(PriceSource.CACHED, book().offline().source)
        assertEquals(0, server.requestCount)
    }

    @Test fun aBrokenServerAnswerOrCopyIsIgnored() = runTest {
        cache.writeText("{ broken")
        server.enqueue(MockResponse.Builder().body("""{"catalogue":{"providers":[]}}""").build())
        assertEquals(PriceSource.BUILT_IN, book().load().source)
        assertTrue("the broken copy is not overwritten with a broken answer", cache.readText() == "{ broken")
    }
}
