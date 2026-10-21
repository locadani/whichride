package io.github.locadani.whichride.server

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class IntegrityTokensTest {
    private val play = IntegrityTokens { "play-token" }
    private val key = "owner-test-key-owner-test-key-owner-test-key"

    @Test fun theKeyWinsOverPlay() = runTest {
        val tokens = chooseIntegrityTokens(key, cloudProjectNumber = 42, play = { play })
        assertTrue(tokens is RegistrationKeyTokens)
        assertEquals(key, tokens.token("hash-1"))
        assertEquals(key, tokens.token("hash-2"))
    }

    @Test fun withoutKeyOrProjectThereIsNoToken() = runTest {
        assertNull(chooseIntegrityTokens("", cloudProjectNumber = 0, play = { play }).token("h"))
    }

    @Test fun withAProjectPlayIsUsed() = runTest {
        assertEquals("play-token", chooseIntegrityTokens("", cloudProjectNumber = 42, play = { play }).token("h"))
    }
}
