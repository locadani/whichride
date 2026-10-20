package io.github.locadani.whichride.options

import io.github.locadani.whichride.core.Choice
import org.junit.Assert.assertEquals
import org.junit.Test

class ChoicesTest {
    private val polo = Choice.Offer("bolt.vw-polo")
    private val yaris = Choice.Offer("bolt.toyota-yaris")
    private val anyBolt = Choice.AnyOf("bolt")
    private val milesS = Choice.Offer("miles.s")
    private val boltOffers = setOf("bolt.vw-polo", "bolt.toyota-yaris", "bolt.toyota-yaris-cross")

    @Test fun tickingACarAddsIt() = assertEquals(setOf(polo, yaris), toggle(setOf(polo), yaris))
    @Test fun tickingItAgainRemovesIt() = assertEquals(setOf(polo), toggle(setOf(polo, yaris), yaris))
    @Test fun anyCarReplacesTheModels() = assertEquals(setOf(anyBolt), toggle(setOf(polo, yaris), anyBolt))
    @Test fun aModelReplacesAnyCar() = assertEquals(setOf(polo), toggle(setOf(anyBolt), polo))
    @Test fun untickingAnyCarLeavesNothing() = assertEquals(emptySet<Choice>(), toggle(setOf(anyBolt), anyBolt))

    @Test fun aProvidersChoicesAreItsOffersAndItsAnyCar() =
        assertEquals(setOf(polo, anyBolt), setOf(polo, anyBolt, milesS).forProvider("bolt", boltOffers))

    @Test fun committingReplacesOnlyThatProvider() = assertEquals(
        setOf(milesS, anyBolt),
        setOf(milesS, polo, yaris).replacingProvider("bolt", boltOffers, setOf(anyBolt)),
    )
}
