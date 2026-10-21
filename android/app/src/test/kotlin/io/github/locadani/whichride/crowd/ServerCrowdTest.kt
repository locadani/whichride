package io.github.locadani.whichride.crowd

import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.Install
import io.github.locadani.whichride.server.Installs
import io.github.locadani.whichride.server.IntegrityTokens
import io.github.locadani.whichride.server.ReportOutcome
import io.github.locadani.whichride.server.ServerApi
import io.github.locadani.whichride.server.TokenStore
import io.github.locadani.whichride.server.writeHash
import kotlinx.coroutines.test.runTest
import mockwebserver3.MockResponse
import mockwebserver3.MockWebServer
import okhttp3.OkHttpClient
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test

class ServerCrowdTest {
    private val server = MockWebServer()
    private val id = "123e4567-e89b-42d3-a456-426614174000"
    private val hashes = mutableListOf<String>()
    private lateinit var crowd: ServerCrowd

    @Before fun start() {
        server.start()
        val api = ServerApi(server.url("/").toString(), OkHttpClient())
        val store = object : TokenStore {
            override fun load() = Install(id, "tok")
            override fun save(install: Install) {}
            override fun clear() {}
        }
        crowd = ServerCrowd(api, Installs(api, IntegrityTokens { hashes += it; "a.b.c.d.e" }, store))
    }

    @After fun stop() = server.close()

    private fun reply(status: Int, body: String = "") =
        server.enqueue(MockResponse.Builder().code(status).body(body).build())

    @Test fun `a vote goes to its report`() = runTest {
        reply(200, """{"status":"accepted"}""")
        val report = "11111111-2222-4333-8444-555555555555"
        assertEquals(ApiResult.Ok(Unit), crowd.vote(report, Answer.NO))
        val request = server.takeRequest()
        assertEquals("/v1/reports/$report/votes", request.url.encodedPath)
        assertEquals("""{"answer":"no"}""", request.body!!.utf8())
        assertEquals(writeHash("POST", "/v1/reports/$report/votes", id, """{"answer":"no"}"""), hashes.single())
    }

    @Test fun `a confirmation answers no content`() = runTest {
        reply(204)
        assertEquals(ApiResult.Ok(Unit), crowd.confirm("miles.s", Answer.YES))
        assertEquals("/v1/offers/miles.s/confirmations", server.takeRequest().url.encodedPath)
    }

    @Test fun `a report sends the core's body byte for byte`() = runTest {
        reply(200, """{"report":null,"status":"current"}""")
        val body = """{"cards":[{"label":"Standard"}],"offer":"miles.s"}"""
        assertEquals(ApiResult.Ok(ReportOutcome(null, "current")), crowd.report(body))
        assertEquals(body, server.takeRequest().body!!.utf8())
    }

    @Test fun `refusals keep their code`() = runTest {
        reply(409, """{"error":"report_closed"}""")
        assertEquals(ApiResult.Refused(409, "report_closed"), crowd.vote("11111111-2222-4333-8444-555555555555", Answer.YES))
    }
}
