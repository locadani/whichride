package io.github.locadani.whichride.i18n

import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.w3c.dom.Element

/** Every string in English exists in every language, with the same placeholders (R17). */
class TranslationsTest {
    private val res = File("src/main/res")
    private val placeholder = Regex("""%(\d+\$)?[sd]|%%""")

    /** name → texts (one for a string, one per quantity for plurals); untranslatable ones left out. */
    private fun read(folder: String): Map<String, Map<String, String>> {
        val document = DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(File(res, "$folder/strings.xml"))
        val result = mutableMapOf<String, Map<String, String>>()
        val strings = document.getElementsByTagName("string")
        for (i in 0 until strings.length) {
            val element = strings.item(i) as Element
            if (element.getAttribute("translatable") == "false") continue
            result[element.getAttribute("name")] = mapOf("" to element.textContent)
        }
        val plurals = document.getElementsByTagName("plurals")
        for (i in 0 until plurals.length) {
            val element = plurals.item(i) as Element
            val items = element.getElementsByTagName("item")
            result["plurals:" + element.getAttribute("name")] =
                (0 until items.length).associate { (items.item(it) as Element).let { item -> item.getAttribute("quantity") to item.textContent } }
        }
        return result
    }

    private fun placeholders(text: String) = placeholder.findAll(text).map { it.value }.toSortedSet()

    @Test fun everyLanguageHasEveryStringWithTheSamePlaceholders() {
        val english = read("values")
        for (language in listOf("it", "de", "tr")) {
            val translated = read("values-$language")
            assertEquals("$language: names", english.keys, translated.keys)
            for ((name, texts) in english) {
                val englishPlaceholders = placeholders(texts["other"] ?: texts.getValue(""))
                for ((quantity, text) in translated.getValue(name)) {
                    assertTrue("$language $name[$quantity] is empty", text.isNotBlank())
                    assertEquals("$language $name[$quantity]", englishPlaceholders, placeholders(text).let { found ->
                        // "one" may spell the number out ("un'auto"), so it may drop the count placeholder.
                        if (quantity == "one") found + englishPlaceholders.filter { it.endsWith("d") } else found
                    }.toSortedSet())
                }
                if (name.startsWith("plurals:")) assertTrue("$language $name has other", "other" in translated.getValue(name))
            }
        }
    }
}
