package io.github.locadani.whichride.personal

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class PersonalStoreTest {
    @get:Rule val folder = TemporaryFolder()

    @Test fun `personal data survives a restart`() {
        val file = File(folder.root, "personal.json")
        val personal = Personal(
            deals = listOf(
                PercentOff("a", "bolt", 2_000, until = 20_757, enabled = true),
                AmountOff("b", "miles", 500, once = true, until = null, used = false, enabled = true),
                Pass("c", "miles", 1_000, 0, 500, waiveUnlock = true, enabled = false),
                RefuelCreditDeal("d", "miles", 300, enabled = true),
            ),
            apps = mapOf("sixt" to "com.sixt.reservation"),
            lastAskedDay = 20_746,
        ).withOverride("miles.s", listOf(card(89)))
        FilePersonalStore(file).save(personal)
        assertEquals(personal, FilePersonalStore(file).load())
    }

    @Test fun `a missing or unreadable file is an empty start`() {
        val file = File(folder.root, "personal.json")
        assertEquals(Personal(), FilePersonalStore(file).load())
        file.writeText("{not json")
        assertEquals(Personal(), FilePersonalStore(file).load())
        FilePersonalStore(file).save(Personal(lastAskedDay = 1))
        assertEquals(Personal(lastAskedDay = 1), FilePersonalStore(file).load())
    }
}
