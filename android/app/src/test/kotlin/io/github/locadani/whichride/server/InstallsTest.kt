package io.github.locadani.whichride.server

import kotlinx.coroutines.test.runTest
import mockwebserver3.MockResponse
import mockwebserver3.MockWebServer
import okhttp3.OkHttpClient
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test

class InstallsTest {
    private val server = MockWebServer()
    private lateinit var api: ServerApi
    private val store = object : TokenStore {
        var token: String? = null
        override fun load() = token
        override fun save(token: String) { this.token = token }
        override fun clear() { token = null }
    }
    private val hashes = mutableListOf<String>()
    private val integrity = IntegrityTokens { hash -> hashes += hash; "a.b.c.d.e" }

    @Before fun start() {
        server.start()
        api = ServerApi(server.url("/").toString().trimEnd('/'), OkHttpClient())
    }

    @After fun stop() = server.close()

    private fun reply(status: Int, body: String) =
        server.enqueue(MockResponse.Builder().code(status).body(body).build())

    private fun installs(tokens: IntegrityTokens = integrity) = Installs(api, tokens, store) { "abcdefghijklmnop" }

    @Test fun registersOnceThenReusesTheStoredToken() = runTest {
        reply(201, """{"install_id":"x","token":"bearer-1"}""")
        val installs = installs()
        assertEquals(ApiResult.Ok("bearer-1"), installs.authorized { ApiResult.Ok(it) })
        assertEquals(ApiResult.Ok("bearer-1"), installs.authorized { ApiResult.Ok(it) })
        assertEquals(1, server.requestCount)
        assertEquals(listOf(registrationHash("""{"nonce":"abcdefghijklmnop"}""")), hashes)
        assertEquals("bearer-1", store.token)
    }

    @Test fun anUnknownTokenRegistersAgainOnce() = runTest {
        store.token = "stale"
        reply(201, """{"install_id":"x","token":"bearer-2"}""")
        val seen = mutableListOf<String>()
        val result = installs().authorized { bearer ->
            seen += bearer
            if (bearer == "stale") ApiResult.Refused(401, "unauthorized") else ApiResult.Ok(bearer)
        }
        assertEquals(ApiResult.Ok("bearer-2"), result)
        assertEquals(listOf("stale", "bearer-2"), seen)
    }

    @Test fun withoutPlayIntegrityThereIsNoInstall() = runTest {
        val result = installs { null }.authorized { ApiResult.Ok(it) }
        assertEquals(ApiResult.Unavailable(Why.NOT_REGISTERED), result)
        assertEquals(0, server.requestCount)
    }

    @Test fun aRefusedRegistrationIsNotRegistered() = runTest {
        reply(403, """{"error":"integrity_failed"}""")
        assertEquals(ApiResult.Unavailable(Why.NOT_REGISTERED), installs().authorized { ApiResult.Ok(it) })
        assertNull(store.token)
    }

    @Test fun aBusyRegistrationIsBusy() = runTest {
        reply(503, """{"error":"integrity_unavailable"}""")
        assertEquals(ApiResult.Unavailable(Why.BUSY), installs().authorized { ApiResult.Ok(it) })
    }
}
