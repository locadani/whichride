package io.github.locadani.whichride.server

import io.github.locadani.whichride.core.Destination
import kotlinx.coroutines.test.runTest
import mockwebserver3.MockResponse
import mockwebserver3.MockWebServer
import okhttp3.OkHttpClient
import org.junit.Assert.assertEquals
import org.junit.Test

class ServerRoutesTest {
    @Test fun expandReturnsTheCoresDestination() = runTest {
        MockWebServer().use { server ->
            server.enqueue(MockResponse.Builder().body("""{"install_id":"x","token":"b"}""").code(201).build())
            server.enqueue(MockResponse.Builder().body("""{"type":"place","query":"Mauerpark","near":null}""").build())
            server.enqueue(MockResponse.Builder().body("""{"type":"nonsense"}""").build())
            server.start()
            val api = ServerApi(server.url("/").toString().trimEnd('/'), OkHttpClient())
            val store = object : TokenStore {
                var t: String? = null
                override fun load() = t
                override fun save(token: String) { t = token }
                override fun clear() { t = null }
            }
            val routes = ServerRoutes(api, Installs(api, { "a.b.c.d.e" }, store) { "abcdefghijklmnop" })
            assertEquals(ApiResult.Ok(Destination.Place("Mauerpark", null)), routes.expand("https://maps.app.goo.gl/x"))
            assertEquals(ApiResult.Refused(200, "invalid_destination"), routes.expand("https://maps.app.goo.gl/y"))
        }
    }
}
