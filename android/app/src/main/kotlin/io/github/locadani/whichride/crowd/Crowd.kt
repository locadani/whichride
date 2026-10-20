package io.github.locadani.whichride.crowd

import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.Installs
import io.github.locadani.whichride.server.ReportOutcome
import io.github.locadani.whichride.server.ServerApi
import io.github.locadani.whichride.server.map
import io.github.locadani.whichride.server.readReportOutcome
import io.github.locadani.whichride.server.readStatus

enum class Answer(val wire: String) { YES("yes"), NO("no") }

/** Crowd prices on the server (§7); every call is a signed write. The ViewModel's tests fake it. */
interface Crowd {
    /** `body` comes from the core (`reportBody`, `proposalBody`) and is sent byte for byte. */
    suspend fun report(body: String): ApiResult<ReportOutcome>
    suspend fun vote(reportId: String, answer: Answer): ApiResult<Unit>
    suspend fun confirm(offerId: String, answer: Answer): ApiResult<Unit>
}

class ServerCrowd(private val api: ServerApi, private val installs: Installs) : Crowd {
    override suspend fun report(body: String) = write(REPORTS, body, ::readReportOutcome)

    override suspend fun vote(reportId: String, answer: Answer) =
        write("/v1/reports/$reportId/votes", answerBody(answer), ::readStatus).map {}

    override suspend fun confirm(offerId: String, answer: Answer) =
        write("/v1/offers/$offerId/confirmations", answerBody(answer)) {}

    private suspend fun <T> write(path: String, body: String, read: (String) -> T): ApiResult<T> =
        installs.signed(path, body) { bearer, token -> api.write(path, body, bearer, token, read) }

    private companion object {
        const val REPORTS = "/v1/reports"
        fun answerBody(answer: Answer) = """{"answer":"${answer.wire}"}"""
    }
}
