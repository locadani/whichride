//! The comparison the app shows: every offer's cheapest price for one trip.

use serde::{Deserialize, Serialize};

use crate::adjustments::Adjustments;
use crate::catalogue::{Catalogue, OfferId, ProviderId};
use crate::money::Cents;
use crate::usage::Trip;

/// The best price of one offer for a trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    pub provider: ProviderId,
    pub offer: OfferId,
    pub offer_name: String,
    /// The tariff that gave the lowest price, e.g. "Standard" or "3 h / 40 km".
    pub card_label: String,
    pub total: Cents,
    /// The price comes from the user's own override, not the shared price.
    pub overridden: bool,
}

/// Prices every offer of every provider and returns them cheapest first.
///
/// Each offer appears once, with its cheapest rate card; on a tie the first card wins. Equal
/// totals are ordered by provider and offer id, so the result is deterministic.
pub fn compare(catalogue: &Catalogue, trip: &Trip, adjustments: &Adjustments) -> Vec<Quote> {
    let mut quotes = Vec::new();
    for provider in &catalogue.providers {
        for offer in &provider.offers {
            let (cards, overridden) = adjustments.cards_for(offer);
            let cheapest = cards
                .iter()
                .map(|card| (card, adjustments.price(provider, card, trip)))
                .min_by_key(|(_, total)| *total);
            if let Some((card, total)) = cheapest {
                quotes.push(Quote {
                    provider: provider.id.clone(),
                    offer: offer.id.clone(),
                    offer_name: offer.name.clone(),
                    card_label: card.label.clone(),
                    total,
                    overridden,
                });
            }
        }
    }
    quotes.sort_by(|a, b| (a.total, &a.provider, &a.offer).cmp(&(b.total, &b.provider, &b.offer)));
    quotes
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::adjustments::{Discount, PriceOverride};
    use crate::catalogue::fixtures::catalogue;
    use crate::money::Percent;

    const TRIP: Trip = Trip {
        metres: 10_200,
        seconds: 25 * 60,
    };

    #[test]
    fn returns_every_offer_cheapest_first() {
        let quotes = compare(&catalogue(), &TRIP, &Adjustments::default());
        let summary: Vec<_> = quotes
            .iter()
            .map(|q| (q.offer.0.as_str(), q.total))
            .collect();
        assert_eq!(
            summary,
            [("bolt.corsa", Cents(473)), ("miles.s", Cents(969))]
        );
    }

    #[test]
    fn picks_the_cheapest_card_and_names_it() {
        let long = Trip {
            metres: 40_000,
            seconds: 170 * 60,
        };
        let quotes = compare(&catalogue(), &long, &Adjustments::default());
        let miles = quotes.iter().find(|q| q.offer.0 == "miles.s").unwrap();
        assert_eq!(miles.card_label, "3 h / 40 km");
        assert_eq!(miles.total, Cents(2_900));
    }

    #[test]
    fn marks_overridden_offers() {
        let c = catalogue();
        let mut card = c.providers[1].offers[0].cards[0].clone();
        card.per_minute = Cents(9);
        let adjustments = Adjustments {
            overrides: vec![PriceOverride {
                offer: OfferId("bolt.corsa".into()),
                cards: vec![card],
            }],
            ..Adjustments::default()
        };
        let quotes = compare(&c, &TRIP, &adjustments);
        let bolt = quotes.iter().find(|q| q.offer.0 == "bolt.corsa").unwrap();
        assert!(bolt.overridden);
        assert_eq!(bolt.total, Cents(11 * 18 + 25 * 9));
        assert!(
            !quotes
                .iter()
                .find(|q| q.offer.0 == "miles.s")
                .unwrap()
                .overridden
        );
    }

    #[test]
    fn empty_override_falls_back_to_the_shared_cards() {
        let adjustments = Adjustments {
            overrides: vec![PriceOverride {
                offer: OfferId("bolt.corsa".into()),
                cards: vec![],
            }],
            ..Adjustments::default()
        };
        let quotes = compare(&catalogue(), &TRIP, &adjustments);
        let bolt = quotes.iter().find(|q| q.offer.0 == "bolt.corsa").unwrap();
        assert_eq!((bolt.total, bolt.overridden), (Cents(473), false));
    }

    #[test]
    fn zero_length_trip_costs_fees_and_minimums_only() {
        let quotes = compare(
            &catalogue(),
            &Trip {
                metres: 0,
                seconds: 0,
            },
            &Adjustments::default(),
        );
        let summary: Vec<_> = quotes
            .iter()
            .map(|q| (q.offer.0.as_str(), q.total))
            .collect();
        assert_eq!(
            summary,
            [("miles.s", Cents(100)), ("bolt.corsa", Cents(250))]
        );
    }

    #[test]
    fn equal_totals_are_ordered_by_provider_then_offer() {
        let mut c = catalogue();
        let card = c.providers[1].offers[0].cards[0].clone();
        c.providers[0].offers[0].cards = vec![card];
        let quotes = compare(&c, &TRIP, &Adjustments::default());
        let order: Vec<_> = quotes.iter().map(|q| q.provider.0.as_str()).collect();
        assert_eq!(order, ["bolt", "miles"]);
    }

    proptest! {
        #[test]
        fn totals_are_never_below_zero_or_the_minimum_and_are_the_cheapest(
            metres in 0u32..2_000_000,
            seconds in 0u32..1_000_000,
            percent in 0u16..=10_000,
            amount in 0i64..100_000,
            refuel in any::<bool>(),
        ) {
            let c = catalogue();
            let trip = Trip { metres, seconds };
            let adjustments = Adjustments {
                discounts: vec![
                    Discount::Percent { provider: c.providers[0].id.clone(), off: Percent::from_basis_points(percent).unwrap() },
                    Discount::Amount { provider: c.providers[1].id.clone(), amount: Cents(amount) },
                ],
                refuel,
                ..Adjustments::default()
            };
            for quote in compare(&c, &trip, &adjustments) {
                prop_assert!(quote.total >= Cents::ZERO);
                let provider = c.provider(&quote.provider).unwrap();
                let offer = provider.offers.iter().find(|o| o.id == quote.offer).unwrap();
                for card in &offer.cards {
                    prop_assert!(quote.total <= adjustments.price(provider, card, &trip));
                }
                let card = offer.cards.iter().find(|card| card.label == quote.card_label).unwrap();
                prop_assert!(quote.total >= card.minimum);
            }
        }
    }
}
