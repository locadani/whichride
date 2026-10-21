//! The catalogue the app compares with: prices, freshness, pending reports, locks and proposals
//! from the server (or the seed in the APK), priced with the user's adjustments.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use carsharing_core as cc;
use serde::Deserialize;

use crate::adjustments::Adjustments;
use crate::cards::{RateCard, from_core};
use crate::{DataError, Trip, invalid};

/// Offer IDs of pending proposals: this prefix and the report ID (plan R14).
pub const PROPOSAL_PREFIX: &str = "proposal.";

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OfferInfo {
    pub id: String,
    pub name: String,
}

/// An app that can open a provider; the provider's default first (decision 66).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AppInfo {
    pub name: String,
    pub android_package: String,
}

/// A provider and its accepted offers (pending proposals are only in `quotes`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub android_package: String,
    pub offers: Vec<OfferInfo>,
    pub apps: Vec<AppInfo>,
}

/// One offer's best price for the trip.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Quote {
    pub provider_id: String,
    pub offer_id: String,
    pub offer_name: String,
    pub card_label: String,
    pub total_cents: i64,
    pub overridden: bool,
    /// Whole days since the crowd last confirmed the price; `None` if never (or built-in prices).
    pub confirmed_days_ago: Option<u32>,
    /// A car proposed by a user and not confirmed yet (§5.1).
    pub proposed: bool,
}

/// The §7.2 question: the values to confirm, and the pending report they belong to (if any).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Question {
    pub offer_id: String,
    pub report_id: Option<String>,
    pub cards: Vec<RateCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum Choice {
    Offer { offer_id: String },
    AnyOf { provider_id: String },
}

impl From<&Choice> for cc::Choice {
    fn from(c: &Choice) -> Self {
        match c {
            Choice::Offer { offer_id } => cc::Choice::Offer {
                offer: cc::OfferId(offer_id.clone()),
            },
            Choice::AnyOf { provider_id } => cc::Choice::AnyOf {
                provider: cc::ProviderId(provider_id.clone()),
            },
        }
    }
}

impl From<cc::Choice> for Choice {
    fn from(c: cc::Choice) -> Self {
        match c {
            cc::Choice::Offer { offer } => Choice::Offer { offer_id: offer.0 },
            cc::Choice::AnyOf { provider } => Choice::AnyOf {
                provider_id: provider.0,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RankedOption {
    pub choice: Choice,
    pub quote: Quote,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Options {
    pub ranked: Vec<RankedOption>,
    pub cheaper_not_added: Vec<Quote>,
}

/// The fields of `GET /v1/catalogue` the app uses.
#[derive(Deserialize)]
struct ServerCatalogue {
    catalogue: cc::Catalogue,
    offers: Vec<ServerOfferStatus>,
    #[serde(default)]
    proposals: Vec<ServerProposal>,
}

#[derive(Deserialize)]
struct ServerOfferStatus {
    offer: cc::OfferId,
    confirmed_on: Option<cc::Day>,
    #[serde(default)]
    locked: bool,
    #[serde(default)]
    pending: Option<ServerPending>,
}

#[derive(Deserialize)]
struct ServerPending {
    report: String,
    cards: Vec<cc::RateCard>,
}

#[derive(Deserialize)]
struct ServerProposal {
    report: String,
    provider: cc::ProviderId,
    name: String,
    cards: Vec<cc::RateCard>,
}

struct Pending {
    report: String,
    cards: Vec<cc::RateCard>,
}

/// A report ID goes into a URL path, so only lowercase hyphenated UUIDs are accepted.
fn is_report_id(id: &str) -> bool {
    id.len() == 36
        && id.chars().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == '-'
            } else {
                c.is_ascii_digit() || ('a'..='f').contains(&c)
            }
        })
}

fn is_proposal(offer: &cc::OfferId) -> bool {
    offer.0.starts_with(PROPOSAL_PREFIX)
}

/// `PROPOSAL_PREFIX` for Kotlin (UniFFI does not export constants); the app checks its copy against it.
#[uniffi::export]
pub fn proposal_prefix() -> String {
    PROPOSAL_PREFIX.into()
}

/// The day number (days since 1970-01-01 UTC) the server and `question` use.
#[uniffi::export]
pub fn day_of(unix_seconds: i64) -> i32 {
    cc::Day::from_unix_seconds(unix_seconds).0
}

/// A validated catalogue with everything the crowd said about it.
#[derive(uniffi::Object)]
pub struct Prices {
    /// Accepted offers only.
    catalogue: cc::Catalogue,
    /// `catalogue` plus pending proposals as offers.
    with_proposals: cc::Catalogue,
    confirmed_on: HashMap<cc::OfferId, cc::Day>,
    pending: HashMap<cc::OfferId, Pending>,
    locked: HashSet<cc::OfferId>,
}

#[uniffi::export]
impl Prices {
    /// The body of the server's `GET /v1/catalogue`. Pending reports and proposals that fail the
    /// core's checks are left out, not errors.
    #[uniffi::constructor]
    pub fn from_server_response(json: String) -> Result<Arc<Prices>, DataError> {
        let response: ServerCatalogue = serde_json::from_str(&json).map_err(invalid)?;
        response.catalogue.validate().map_err(invalid)?;
        let mut confirmed_on = HashMap::new();
        let mut pending = HashMap::new();
        let mut locked = HashSet::new();
        for status in response.offers {
            if let Some(day) = status.confirmed_on {
                confirmed_on.insert(status.offer.clone(), day);
            }
            if status.locked {
                locked.insert(status.offer.clone());
            }
            if let Some(report) = status.pending
                && is_report_id(&report.report)
                && cc::catalogue::validate_cards(&status.offer, &report.cards).is_ok()
            {
                pending.insert(
                    status.offer,
                    Pending {
                        report: report.report,
                        cards: report.cards,
                    },
                );
            }
        }
        let mut with_proposals = response.catalogue.clone();
        for proposal in response.proposals {
            let id = cc::OfferId(format!("{PROPOSAL_PREFIX}{}", proposal.report));
            let usable = is_report_id(&proposal.report)
                && cc::catalogue::validate_cards(&id, &proposal.cards).is_ok();
            if usable
                && let Some(provider) = with_proposals
                    .providers
                    .iter_mut()
                    .find(|p| p.id == proposal.provider)
            {
                provider.offers.push(cc::Offer {
                    id,
                    name: proposal.name,
                    cards: proposal.cards,
                });
            }
        }
        Ok(Arc::new(Prices {
            catalogue: response.catalogue,
            with_proposals,
            confirmed_on,
            pending,
            locked,
        }))
    }

    /// The seed catalogue shipped in the APK (`crates/server/catalogue.json`); nothing is confirmed.
    #[uniffi::constructor]
    pub fn from_seed(json: String) -> Result<Arc<Prices>, DataError> {
        let catalogue = cc::Catalogue::from_json(&json).map_err(invalid)?;
        Ok(Arc::new(Prices {
            with_proposals: catalogue.clone(),
            catalogue,
            confirmed_on: HashMap::new(),
            pending: HashMap::new(),
            locked: HashSet::new(),
        }))
    }

    pub fn providers(&self) -> Vec<ProviderInfo> {
        self.catalogue
            .providers
            .iter()
            .map(|p| ProviderInfo {
                id: p.id.0.clone(),
                name: p.name.clone(),
                android_package: p.android_package.clone(),
                offers: p
                    .offers
                    .iter()
                    .map(|o| OfferInfo {
                        id: o.id.0.clone(),
                        name: o.name.clone(),
                    })
                    .collect(),
                apps: p
                    .apps()
                    .into_iter()
                    .map(|a| AppInfo {
                        name: a.name,
                        android_package: a.android_package,
                    })
                    .collect(),
            })
            .collect()
    }

    /// Every offer's best price with the user's adjustments, cheapest first, pending proposals
    /// included (marked `proposed`).
    pub fn quotes(
        &self,
        trip: Trip,
        adjustments: Adjustments,
        today_unix_seconds: i64,
    ) -> Vec<Quote> {
        let today = cc::Day::from_unix_seconds(today_unix_seconds);
        self.core_quotes(trip, &adjustments)
            .iter()
            .map(|q| self.quote(q, today))
            .collect()
    }

    /// The user's added cars ranked, and the cheaper ones they did not add (spec §4.1). A
    /// proposal counts only when the user added it (plan R14).
    pub fn options(
        &self,
        trip: Trip,
        choices: Vec<Choice>,
        adjustments: Adjustments,
        today_unix_seconds: i64,
    ) -> Options {
        let today = cc::Day::from_unix_seconds(today_unix_seconds);
        let added: HashSet<&str> = choices
            .iter()
            .filter_map(|c| match c {
                Choice::Offer { offer_id } => Some(offer_id.as_str()),
                Choice::AnyOf { .. } => None,
            })
            .collect();
        let quotes: Vec<cc::Quote> = self
            .core_quotes(trip, &adjustments)
            .into_iter()
            .filter(|q| !is_proposal(&q.offer) || added.contains(q.offer.0.as_str()))
            .collect();
        let choices: Vec<cc::Choice> = choices.iter().map(Into::into).collect();
        let options = cc::options(&quotes, &choices);
        Options {
            ranked: options
                .ranked
                .into_iter()
                .map(|o| RankedOption {
                    choice: o.choice.into(),
                    quote: self.quote(&o.quote, today),
                })
                .collect(),
            cheaper_not_added: options
                .cheaper_not_added
                .iter()
                .map(|q| self.quote(q, today))
                .collect(),
        }
    }

    /// The shared rate cards of an offer or proposal; empty if unknown.
    pub fn cards(&self, offer_id: String) -> Vec<RateCard> {
        self.offer(&cc::OfferId(offer_id))
            .map(|o| from_core(&o.cards))
            .unwrap_or_default()
    }

    /// The owner locked or hid this offer: no reports, no questions.
    pub fn is_locked(&self, offer_id: String) -> bool {
        self.locked.contains(&cc::OfferId(offer_id))
    }

    /// The §7.2 question for an offer the user just opened, if one is due (plan R12): about a
    /// proposal itself, else about the oldest pending report (decision 37), else about the
    /// current price when it was not confirmed recently. `last_asked_day` is a `day_of` value.
    pub fn question(
        &self,
        offer_id: String,
        today_unix_seconds: i64,
        last_asked_day: Option<i32>,
    ) -> Option<Question> {
        let offer = cc::OfferId(offer_id);
        if self.locked.contains(&offer) {
            return None;
        }
        let current = self.offer(&offer)?;
        let (report_id, cards) = if let Some(report) = offer.0.strip_prefix(PROPOSAL_PREFIX) {
            (Some(report.to_owned()), &current.cards)
        } else if let Some(pending) = self.pending.get(&offer) {
            (Some(pending.report.clone()), &pending.cards)
        } else {
            (None, &current.cards)
        };
        let ask = cc::should_ask(
            cc::Day::from_unix_seconds(today_unix_seconds),
            last_asked_day.map(cc::Day),
            report_id.is_some(),
            self.confirmed_on.get(&offer).copied(),
        );
        ask.then(|| Question {
            offer_id: offer.0.clone(),
            report_id,
            cards: from_core(cards),
        })
    }
}

impl Prices {
    fn offer(&self, id: &cc::OfferId) -> Option<&cc::Offer> {
        self.with_proposals
            .providers
            .iter()
            .flat_map(|p| &p.offers)
            .find(|o| &o.id == id)
    }

    fn core_quotes(&self, trip: Trip, adjustments: &Adjustments) -> Vec<cc::Quote> {
        cc::compare(&self.with_proposals, &trip.into(), &adjustments.into())
    }

    fn quote(&self, q: &cc::Quote, today: cc::Day) -> Quote {
        Quote {
            provider_id: q.provider.0.clone(),
            offer_id: q.offer.0.clone(),
            offer_name: q.offer_name.clone(),
            card_label: q.card_label.clone(),
            total_cents: q.total.0,
            overridden: q.overridden,
            confirmed_days_ago: cc::confirmed_days_ago(
                today,
                self.confirmed_on.get(&q.offer).copied(),
            ),
            proposed: is_proposal(&q.offer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &str = include_str!("../../server/catalogue.json");
    /// 2026-10-20 12:00 UTC = day 20_746.
    const TODAY: i64 = 1_792_497_600;

    const TRIP: Trip = Trip {
        metres: 7_400,
        seconds: 22 * 60,
    };

    const PENDING: &str = "11111111-2222-4333-8444-555555555555";
    const PROPOSAL: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";

    fn card_json(per_km: i64) -> serde_json::Value {
        serde_json::json!({"label":"Standard","kind":"standard","base_fee":100,"per_km":per_km,
            "per_minute":0,"km_rounding":"started_unit","minute_rounding":"started_unit"})
    }

    fn server_response() -> String {
        let catalogue: serde_json::Value = serde_json::from_str(SEED).unwrap();
        serde_json::json!({
            "catalogue": catalogue,
            "offers": [
                {"offer": "miles.s", "confirmed_on": 20_744, "locked": false,
                 "pending": {"report": PENDING, "cards": [card_json(89)]}},
                {"offer": "miles.m", "confirmed_on": 20_746, "locked": false, "pending": null},
                {"offer": "bolt.vw-polo", "confirmed_on": null, "locked": true, "pending": null}
            ],
            "proposals": [
                {"report": PROPOSAL, "provider": "bolt", "name": "Fiat 500e", "cards": [card_json(1)]},
                {"report": "../../etc", "provider": "bolt", "name": "Bad id", "cards": [card_json(1)]},
                {"report": PROPOSAL, "provider": "nobody", "name": "Unknown provider", "cards": [card_json(1)]}
            ]
        })
        .to_string()
    }

    fn server() -> Arc<Prices> {
        Prices::from_server_response(server_response()).unwrap()
    }

    #[test]
    fn proposals_are_quoted_but_count_only_when_added() {
        let prices = server();
        let proposal_id = format!("{PROPOSAL_PREFIX}{PROPOSAL}");
        let quotes = prices.quotes(TRIP, Adjustments::default(), TODAY);
        let proposed: Vec<_> = quotes.iter().filter(|q| q.proposed).collect();
        assert_eq!(
            proposed.len(),
            1,
            "bad report IDs and unknown providers are left out"
        );
        assert_eq!(proposed[0].offer_id, proposal_id);
        assert!(
            prices.providers()[1]
                .offers
                .iter()
                .all(|o| !o.id.starts_with(PROPOSAL_PREFIX))
        );

        let any_bolt = vec![Choice::AnyOf {
            provider_id: "bolt".into(),
        }];
        let options = prices.options(TRIP, any_bolt, Adjustments::default(), TODAY);
        assert!(
            !options.ranked[0].quote.proposed,
            "any car never resolves to a proposal"
        );
        assert!(options.cheaper_not_added.iter().all(|q| !q.proposed));

        let added = vec![Choice::Offer {
            offer_id: proposal_id.clone(),
        }];
        let options = prices.options(TRIP, added, Adjustments::default(), TODAY);
        assert_eq!(options.ranked[0].quote.offer_id, proposal_id);
    }

    #[test]
    fn adjustments_change_the_quotes() {
        let prices = Prices::from_seed(SEED.into()).unwrap();
        let plain = prices.quotes(TRIP, Adjustments::default(), TODAY);
        let miles_s = |quotes: &[Quote]| {
            quotes
                .iter()
                .find(|q| q.offer_id == "miles.s")
                .unwrap()
                .clone()
        };
        let half = Adjustments {
            discounts: vec![crate::adjustments::Discount::Percent {
                provider_id: "miles".into(),
                off: 5_000,
            }],
            ..Adjustments::default()
        };
        let discounted = miles_s(&prices.quotes(TRIP, half, TODAY));
        assert_eq!(
            discounted.total_cents,
            (miles_s(&plain).total_cents + 1) / 2
        );
        let mine = Adjustments {
            overrides: vec![crate::adjustments::PriceOverride {
                offer_id: "miles.s".into(),
                cards: crate::cards::cards_from_json(format!("[{}]", card_json(10))).unwrap(),
            }],
            ..Adjustments::default()
        };
        assert!(miles_s(&prices.quotes(TRIP, mine, TODAY)).overridden);
    }

    #[test]
    fn the_question_is_about_the_pending_report_first() {
        let prices = server();
        let question = prices.question("miles.s".into(), TODAY, None).unwrap();
        assert_eq!(question.report_id.as_deref(), Some(PENDING));
        assert_eq!(question.cards[0].per_km, 89);
    }

    #[test]
    fn no_question_about_locked_unknown_or_recently_confirmed_offers() {
        let prices = server();
        assert_eq!(prices.question("bolt.vw-polo".into(), TODAY, None), None);
        assert_eq!(prices.question("nothing.here".into(), TODAY, None), None);
        assert_eq!(
            prices.question("miles.m".into(), TODAY, None),
            None,
            "confirmed today"
        );
        assert!(prices.is_locked("bolt.vw-polo".into()));
    }

    #[test]
    fn the_question_waits_a_week_after_the_last_one() {
        let prices = server();
        let today = day_of(TODAY);
        assert_eq!(
            prices.question("miles.s".into(), TODAY, Some(today - 6)),
            None
        );
        assert!(
            prices
                .question("miles.s".into(), TODAY, Some(today - 7))
                .is_some()
        );
    }

    #[test]
    fn a_proposal_question_votes_on_the_proposal() {
        let prices = server();
        let question = prices
            .question(format!("{PROPOSAL_PREFIX}{PROPOSAL}"), TODAY, None)
            .unwrap();
        assert_eq!(question.report_id.as_deref(), Some(PROPOSAL));
        assert_eq!(question.cards[0].per_km, 1);
    }

    #[test]
    fn cards_of_unknown_offers_are_empty() {
        let prices = server();
        assert_eq!(prices.cards("miles.s".into())[0].per_km, 99);
        assert!(prices.cards("nope".into()).is_empty());
    }

    #[test]
    fn the_seed_catalogue_loads_with_every_v1_provider() {
        let prices = Prices::from_seed(SEED.into()).unwrap();
        let ids: Vec<_> = prices.providers().into_iter().map(|p| p.id).collect();
        assert_eq!(ids, ["miles", "bolt", "sixt"]);
        assert_eq!(
            prices.providers()[2].android_package,
            "com.trafi.whitelabel.bvg"
        );
        assert_eq!(
            prices.providers()[2]
                .apps
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>(),
            ["BVG Jelbi", "SIXT"]
        );
    }

    #[test]
    fn seed_quotes_have_no_confirmation_date() {
        let prices = Prices::from_seed(SEED.into()).unwrap();
        let quotes = prices.quotes(TRIP, Adjustments::default(), TODAY);
        assert_eq!(quotes.len(), 9);
        assert!(
            quotes
                .windows(2)
                .all(|w| w[0].total_cents <= w[1].total_cents)
        );
        assert!(quotes.iter().all(|q| q.confirmed_days_ago.is_none()));
    }

    #[test]
    fn server_quotes_say_how_long_ago_they_were_confirmed() {
        let prices = Prices::from_server_response(server_response()).unwrap();
        let quotes = prices.quotes(TRIP, Adjustments::default(), TODAY);
        let days = |id: &str| {
            quotes
                .iter()
                .find(|q| q.offer_id == id)
                .unwrap()
                .confirmed_days_ago
        };
        assert_eq!(days("miles.s"), Some(2));
        assert_eq!(days("bolt.vw-polo"), None);
    }

    #[test]
    fn options_cross_the_boundary() {
        let prices = Prices::from_seed(SEED.into()).unwrap();
        let options = prices.options(
            TRIP,
            vec![
                Choice::AnyOf {
                    provider_id: "miles".into(),
                },
                Choice::Offer {
                    offer_id: "sixt.volkswagen-t-roc".into(),
                },
            ],
            Adjustments::default(),
            TODAY,
        );
        assert_eq!(options.ranked.len(), 2);
        let miles = Choice::AnyOf {
            provider_id: "miles".into(),
        };
        assert!(options.ranked.iter().any(|o| o.choice == miles));
        assert!(options.ranked[0].quote.total_cents <= options.ranked[1].quote.total_cents);
    }

    #[test]
    fn invalid_catalogues_are_errors_not_panics() {
        assert!(Prices::from_seed("{".into()).is_err());
        assert!(
            Prices::from_seed(
                r#"{"providers":[{"id":"x","name":"X","android_package":"x","offers":[]}]}"#.into()
            )
            .is_err()
        );
        assert!(Prices::from_server_response(SEED.into()).is_err());
    }
}
