//! The cars near the user that they added as options (spec §4.1): ranked by price, plus the
//! offers they did not add that would be cheaper.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::catalogue::{OfferId, ProviderId};
use crate::compare::Quote;

/// One car the user added: a specific offer, or "any car" of a provider when they don't know
/// the model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Choice {
    Offer { offer: OfferId },
    AnyOf { provider: ProviderId },
}

/// An added car with its price; "any car" is priced as the provider's cheapest offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedOption {
    pub choice: Choice,
    pub quote: Quote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Cheapest first; empty when nothing was added.
    pub ranked: Vec<RankedOption>,
    /// Offers not added that cost strictly less than the cheapest option, cheapest first.
    pub cheaper_not_added: Vec<Quote>,
}

/// Ranks the user's choices. `quotes` must be in `compare()`'s order (cheapest first, ties by
/// provider and offer), so walking them yields the ranking and each provider's first quote is
/// its cheapest. "Any car" covers every offer of its provider; choices for offers or providers
/// missing from the quotes (removed from the catalogue since) are skipped.
pub fn options(quotes: &[Quote], choices: &[Choice]) -> Options {
    let any_of: HashSet<&ProviderId> = choices
        .iter()
        .filter_map(|choice| match choice {
            Choice::AnyOf { provider } => Some(provider),
            Choice::Offer { .. } => None,
        })
        .collect();
    let offers: HashSet<&OfferId> = choices
        .iter()
        .filter_map(|choice| match choice {
            Choice::Offer { offer } => Some(offer),
            Choice::AnyOf { .. } => None,
        })
        .collect();

    let mut ranked = Vec::new();
    let mut providers_seen = HashSet::new();
    for quote in quotes {
        let choice = if any_of.contains(&quote.provider) {
            if !providers_seen.insert(&quote.provider) {
                continue;
            }
            Choice::AnyOf {
                provider: quote.provider.clone(),
            }
        } else if offers.contains(&quote.offer) {
            Choice::Offer {
                offer: quote.offer.clone(),
            }
        } else {
            continue;
        };
        ranked.push(RankedOption {
            choice,
            quote: quote.clone(),
        });
    }

    // The cheapest option is the cheapest added offer, so everything strictly cheaper was not added.
    let cheaper_not_added = match ranked.first() {
        Some(best) => quotes
            .iter()
            .take_while(|quote| quote.total < best.quote.total)
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    Options {
        ranked,
        cheaper_not_added,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Cents;

    fn quote(provider: &str, offer: &str, total: i64) -> Quote {
        Quote {
            provider: ProviderId(provider.into()),
            offer: OfferId(offer.into()),
            offer_name: offer.into(),
            card_label: "Standard".into(),
            total: Cents(total),
            overridden: false,
        }
    }

    /// Cheapest first, as `compare()` returns them.
    fn quotes() -> Vec<Quote> {
        vec![
            quote("bolt", "bolt.picanto", 790),
            quote("miles", "miles.s", 849),
            quote("bolt", "bolt.polo", 912),
            quote("miles", "miles.premium", 1180),
            quote("sixt", "sixt.troc", 1300),
        ]
    }

    fn offer(id: &str) -> Choice {
        Choice::Offer {
            offer: OfferId(id.into()),
        }
    }

    fn any(id: &str) -> Choice {
        Choice::AnyOf {
            provider: ProviderId(id.into()),
        }
    }

    fn ranked(options: &Options) -> Vec<(&str, i64)> {
        options
            .ranked
            .iter()
            .map(|o| (o.quote.offer.0.as_str(), o.quote.total.0))
            .collect()
    }

    #[test]
    fn nothing_added_ranks_nothing_and_reports_nothing() {
        let options = options(&quotes(), &[]);
        assert!(options.ranked.is_empty());
        assert!(options.cheaper_not_added.is_empty());
    }

    #[test]
    fn added_offers_are_ranked_cheapest_first() {
        let options = options(
            &quotes(),
            &[offer("sixt.troc"), offer("bolt.polo"), offer("miles.s")],
        );
        assert_eq!(
            ranked(&options),
            [("miles.s", 849), ("bolt.polo", 912), ("sixt.troc", 1300)]
        );
    }

    #[test]
    fn any_car_is_the_providers_cheapest_offer() {
        let options = options(&quotes(), &[any("miles"), offer("sixt.troc")]);
        assert_eq!(ranked(&options), [("miles.s", 849), ("sixt.troc", 1300)]);
        assert_eq!(options.ranked[0].choice, any("miles"));
    }

    #[test]
    fn any_car_covers_that_providers_specific_offers() {
        let options = options(&quotes(), &[offer("miles.premium"), any("miles")]);
        assert_eq!(ranked(&options), [("miles.s", 849)]);
    }

    #[test]
    fn unknown_offers_are_skipped() {
        let options = options(
            &quotes(),
            &[offer("bolt.gone"), any("free2move"), offer("bolt.polo")],
        );
        assert_eq!(ranked(&options), [("bolt.polo", 912)]);
    }

    #[test]
    fn a_choice_added_twice_appears_once() {
        let options = options(&quotes(), &[offer("bolt.polo"), offer("bolt.polo")]);
        assert_eq!(ranked(&options), [("bolt.polo", 912)]);
    }

    #[test]
    fn cheaper_offers_not_added_are_listed_cheapest_first() {
        let options = options(&quotes(), &[offer("miles.premium")]);
        let cheaper: Vec<_> = options
            .cheaper_not_added
            .iter()
            .map(|q| q.offer.0.as_str())
            .collect();
        assert_eq!(cheaper, ["bolt.picanto", "miles.s", "bolt.polo"]);
    }

    #[test]
    fn an_offer_with_the_same_price_is_not_cheaper() {
        let mut quotes = quotes();
        quotes.insert(2, quote("sixt", "sixt.polo", 849));
        let options = options(&quotes, &[offer("miles.s")]);
        let cheaper: Vec<_> = options
            .cheaper_not_added
            .iter()
            .map(|q| q.offer.0.as_str())
            .collect();
        assert_eq!(cheaper, ["bolt.picanto"]);
    }

    #[test]
    fn nothing_is_cheaper_than_the_cheapest_offer() {
        let options = options(&quotes(), &[any("bolt")]);
        assert!(options.cheaper_not_added.is_empty());
    }

    #[test]
    fn choices_round_trip_through_json() {
        let json = serde_json::to_string(&[offer("bolt.polo"), any("miles")]).unwrap();
        assert_eq!(
            json,
            r#"[{"type":"offer","offer":"bolt.polo"},{"type":"any_of","provider":"miles"}]"#
        );
        let back: Vec<Choice> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, [offer("bolt.polo"), any("miles")]);
    }
}
