package io.github.locadani.whichride.ui

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.QuestionState
import io.github.locadani.whichride.compare.QuestionStep
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.CardKind
import io.github.locadani.whichride.core.Question
import io.github.locadani.whichride.core.RateCard
import io.github.locadani.whichride.core.Rounding
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.crowd.PricePurpose
import io.github.locadani.whichride.designsystem.WhichrideTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class QuestionSheetTest {
    @get:Rule val rule = createComposeRule()
    private val card = RateCard("Standard", CardKind.STANDARD, 100, 29, 11, 0u, 0u, 0, Rounding.STARTED_UNIT, Rounding.STARTED_UNIT)
    private fun state(offerId: String = "bolt.polo", step: QuestionStep = QuestionStep.ASK) = UiState(
        providers = PROVIDERS,
        question = QuestionState(Question(offerId, null, listOf(card)), "bolt", "VW Polo", step),
        priceForm = if (step == QuestionStep.FIX) PriceForm(PricePurpose.Report(offerId, "bolt", "VW Polo"), unlock = "1", perKm = "0.29", perMinute = "0.11", minimum = "0") else null,
    )

    @Test fun `the question shows the values and the three answers`() {
        val pressed = mutableListOf<String>()
        rule.setContent {
            WhichrideTheme {
                QuestionContent(state(), QuestionActions(onYes = { pressed += "yes" }, onNo = { pressed += "no" }, onClose = { pressed += "skip" }))
            }
        }
        rule.onNodeWithText("Is this the price Bolt Drive showed for the VW Polo?").assertExists()
        rule.onNodeWithText("€0.29").assertExists()
        rule.onNodeWithText("€0.11").assertExists()
        rule.onNodeWithText("€1.00").assertExists()
        rule.onNodeWithText("Yes, that's right").performClick()
        rule.onNodeWithText("No, fix it").performClick()
        rule.onNodeWithText("Skip").performClick()
        assertEquals(listOf("yes", "no", "skip"), pressed)
    }

    @Test fun `a proposal is a plain yes or no`() {
        rule.setContent { WhichrideTheme { QuestionContent(state(offerId = "proposal.x"), QuestionActions()) } }
        rule.onNodeWithText("Did Bolt Drive show the VW Polo at these prices?").assertExists()
        rule.onNodeWithText("No").assertExists()
        rule.onNodeWithText("No, fix it").assertDoesNotExist()
    }

    @Test fun `fixing offers the form and I don't know`() {
        var dontKnow = false
        rule.setContent { WhichrideTheme { QuestionContent(state(step = QuestionStep.FIX), QuestionActions(onDontKnow = { dontKnow = true })) } }
        rule.onNodeWithText("What does Bolt Drive show?").assertExists()
        rule.onNodeWithText("Per km (€)").assertExists()
        rule.onNodeWithText("I don't know the numbers").performScrollTo().performClick()
        assertEquals(true, dontKnow)
    }

    @Test fun `the end says what happened`() {
        rule.setContent { WhichrideTheme { QuestionContent(state(step = QuestionStep.NOT_SENT), QuestionActions()) } }
        rule.onNodeWithText("We couldn't send your answer now. Nothing changed.").assertExists()
        rule.onNodeWithText("Close").assertExists()
    }
}
