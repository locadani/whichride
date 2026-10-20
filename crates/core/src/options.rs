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
