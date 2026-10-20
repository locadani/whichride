package io.github.locadani.whichride.server

import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Trip
import kotlinx.coroutines.test.runTest
import mockwebserver3.MockResponse
import mockwebserver3.MockWebServer
import okhttp3.OkHttpClient
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Before
import org.junit.Test

class ServerApiTest {
    private val server = MockWebServer()
    private lateinit var api: ServerApi
    private val alex = Coordinate(525_219_000, 134_132_000)
    private val neun = Coordinate(525_022_870, 134_316_050)

    @Before fun start() {
        server.start()
        api = ServerApi(server.url("/").toString().trimEnd('/'), OkHttpClient())
    }

    @After fun stop() = server.close()

    private fun reply(status: Int, body: String) =
        server.enqueue(MockResponse.Builder().code(status).body(body).build())

    @Test fun routeSendsCoordinatesInTheBodyWithTheBearer() = runTest {
        reply(200, """{"metres":7400,"seconds":1320}""")
        assertEquals(ApiResult.Ok(Trip(7_400u, 1_320u)), api.route("tok", alex, neun))
        val request = server.takeRequest()
        assertEquals("POST", request.method)
        assertEquals("/v1/route", request.url.encodedPath)
        assertFalse("no coordinates in the URL (§9)", request.url.toString().contains("52"))
        assertEquals("Bearer tok", request.headers["Authorization"])
        assertEquals(
            """{"from":{"lat_e7":525219000,"lon_e7":134132000},"to":{"lat_e7":525022870,"lon_e7":134316050}}""",
            request.body!!.utf8(),
        )
    }

    @Test fun searchesSendNearOnlyWhenKnown() = runTest {
        reply(200, """{"places":[{"label":"Markthalle Neun","at":{"lat_e7":525022870,"lon_e7":134316050}}]}""")
        reply(200, """{"places":[]}""")
        assertEquals(ApiResult.Ok(listOf(Place("Markthalle Neun", neun))), api.autocomplete("tok", "Markth", alex))
        assertEquals(ApiResult.Ok(emptyList<Place>()), api.geocode("tok", "Nowhere", null))
        assertEquals("""{"text":"Markth","near":{"lat_e7":525219000,"lon_e7":134132000}}""", server.takeRequest().body!!.utf8())
        val geocode = server.takeRequest()
        assertEquals("/v1/geocode", geocode.url.encodedPath)
        assertEquals("""{"text":"Nowhere"}""", geocode.body!!.utf8())
    }

    @Test fun errorsMapToWhatTheAppDoesNext() = runTest {
        reply(503, """{"error":"route_service_busy"}""")
        reply(429, """{"error":"too_many_requests"}""")
        reply(500, """{"error":"internal"}""")
        reply(422, """{"error":"no_route"}""")
        reply(401, """{"error":"unauthorized"}""")
        reply(422, "not json")
        assertEquals(ApiResult.Unavailable(Why.BUSY), api.route("t", alex, neun))
        assertEquals(ApiResult.Unavailable(Why.BUSY), api.route("t", alex, neun))
        assertEquals(ApiResult.Unavailable(Why.BUSY), api.route("t", alex, neun))
        assertEquals(ApiResult.Refused(422, "no_route"), api.route("t", alex, neun))
        assertEquals(ApiResult.Refused(401, "unauthorized"), api.route("t", alex, neun))
        assertEquals(ApiResult.Refused(422, "unknown"), api.route("t", alex, neun))
    }

    @Test fun aServerThatIsDownIsOffline() = runTest {
        server.close()
        assertEquals(ApiResult.Unavailable(Why.OFFLINE), api.route("t", alex, neun))
    }

    @Test fun noServerUrlMeansNotConfigured() = runTest {
        val none = ServerApi("", OkHttpClient())
        assertEquals(ApiResult.Unavailable(Why.NOT_CONFIGURED), none.catalogue())
        assertEquals(ApiResult.Unavailable(Why.NOT_CONFIGURED), none.route("t", alex, neun))
    }

    @Test fun catalogueAndExpandReturnTheRawJsonForTheCore() = runTest {
        reply(200, """{"catalogue":{"providers":[]},"offers":[],"proposals":[]}""")
        reply(200, """{"type":"place","query":"Mauerpark","near":null}""")
        assertEquals(ApiResult.Ok("""{"catalogue":{"providers":[]},"offers":[],"proposals":[]}"""), api.catalogue())
        assertEquals(ApiResult.Ok("""{"type":"place","query":"Mauerpark","near":null}"""), api.expand("t", "https://maps.app.goo.gl/x"))
        assertEquals("GET", server.takeRequest().method)
        assertEquals("""{"url":"https://maps.app.goo.gl/x"}""", server.takeRequest().body!!.utf8())
    }

    @Test fun registrationSendsTheIntegrityTokenAndReturnsTheBearer() = runTest {
        reply(201, """{"install_id":"123e4567-e89b-42d3-a456-426614174000","token":"bearer-1"}""")
        assertEquals(ApiResult.Ok("bearer-1"), api.register("""{"nonce":"abcdefghijklmnop"}""", "int.tok.en.x.y"))
        val request = server.takeRequest()
        assertEquals("int.tok.en.x.y", request.headers["X-Integrity-Token"])
        assertEquals("""{"nonce":"abcdefghijklmnop"}""", request.body!!.utf8())
    }

    @Test fun registrationHashMatchesTheServersKnownValue() = assertEquals(
        "c59b7c5ccbba73f1c1f2c526838ae9486eaaf2ac726b320a170aad41b31565c3",
        registrationHash("""{"nonce":"abcdefghijklmnop"}"""),
    )
}
