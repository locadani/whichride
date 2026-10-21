//! The website's view of the core (website spec §4.1): every offer priced for a typed trip, and
//! the visitor's ticked cars ranked. No personal adjustments: the web has no discounts.

use carsharing_core as cc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub offers: Vec<OfferInfo>,
}

/// One offer's best price for the trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Priced {
    pub provider_id: String,
    pub offer_id: String,
    pub offer_name: String,
    /// The tariff that gave the price, e.g. "Standard" or "3 h / 40 km".
    pub card_label: String,
    pub total_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ranking {
    pub ranked: Vec<Priced>,
    pub cheaper_not_added: Vec<Priced>,
}

impl From<&cc::Quote> for Priced {
    fn from(q: &cc::Quote) -> Self {
        Priced {
            provider_id: q.provider.0.clone(),
            offer_id: q.offer.0.clone(),
            offer_name: q.offer_name.clone(),
            card_label: q.card_label.clone(),
            total_cents: q.total.0,
        }
    }
}

/// The one field of `GET /v1/catalogue` the website uses.
#[derive(Deserialize)]
struct ServerCatalogue {
    catalogue: cc::Catalogue,
}

#[derive(Debug, Clone)]
pub struct WebPrices {
    catalogue: cc::Catalogue,
}

impl WebPrices {
    /// The body of `GET /v1/catalogue`: accepted prices only; pending reports and proposals are
    /// the app's (the web is read-only, spec §8.1).
    pub fn from_server(json: &str) -> Result<WebPrices, String> {
        let response: ServerCatalogue = serde_json::from_str(json).map_err(|e| e.to_string())?;
        response.catalogue.validate().map_err(|e| e.to_string())?;
        Ok(WebPrices {
            catalogue: response.catalogue,
        })
    }

    /// The seed catalogue (`crates/server/catalogue.json`) built into the site.
    pub fn from_seed(json: &str) -> Result<WebPrices, String> {
        cc::Catalogue::from_json(json)
            .map(|catalogue| WebPrices { catalogue })
            .map_err(|e| e.to_string())
    }

    /// Providers and their offers in catalogue order (the sheet's order).
    pub fn providers(&self) -> Vec<ProviderInfo> {
        self.catalogue
            .providers
            .iter()
            .map(|p| ProviderInfo {
                id: p.id.0.clone(),
                name: p.name.clone(),
                offers: p
                    .offers
                    .iter()
                    .map(|o| OfferInfo {
                        id: o.id.0.clone(),
                        name: o.name.clone(),
                    })
                    .collect(),
            })
            .collect()
    }

    /// Every offer's price for the trip, in `compare()` order.
    pub fn offers(&self, metres: u32, seconds: u32) -> Vec<Priced> {
        self.quotes(metres, seconds)
            .iter()
            .map(Priced::from)
            .collect()
    }

    /// The ticked offers ranked, and the cheaper ones not ticked (the core's `options()`).
    /// Unknown offer IDs are skipped.
    pub fn options(&self, metres: u32, seconds: u32, offer_ids: &[String]) -> Ranking {
        let choices: Vec<cc::Choice> = offer_ids
            .iter()
            .map(|id| cc::Choice::Offer {
                offer: cc::OfferId(id.clone()),
            })
            .collect();
        let options = cc::options(&self.quotes(metres, seconds), &choices);
        Ranking {
            ranked: options
                .ranked
                .iter()
                .map(|o| Priced::from(&o.quote))
                .collect(),
            cheaper_not_added: options.cheaper_not_added.iter().map(Priced::from).collect(),
        }
    }

    fn quotes(&self, metres: u32, seconds: u32) -> Vec<cc::Quote> {
        let trip = cc::Trip { metres, seconds };
        cc::compare(&self.catalogue, &trip, &cc::Adjustments::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &str = include_str!("../../server/catalogue.json");
    const METRES: u32 = 7_400;
    const SECONDS: u32 = 22 * 60;

    fn seed() -> WebPrices {
        WebPrices::from_seed(SEED).unwrap()
    }

    fn core_quotes(catalogue: &cc::Catalogue) -> Vec<cc::Quote> {
        let trip = cc::Trip {
            metres: METRES,
            seconds: SECONDS,
        };
        cc::compare(catalogue, &trip, &cc::Adjustments::default())
    }

    fn ids(priced: &[Priced]) -> Vec<&str> {
        priced.iter().map(|p| p.offer_id.as_str()).collect()
    }

    #[test]
    fn every_offer_is_priced_in_the_cores_order() {
        let core = core_quotes(&cc::Catalogue::from_json(SEED).unwrap());
        let offers = seed().offers(METRES, SECONDS);
        let expected: Vec<(&str, i64)> = core
            .iter()
            .map(|q| (q.offer.0.as_str(), q.total.0))
            .collect();
        let got: Vec<(&str, i64)> = offers
            .iter()
            .map(|o| (o.offer_id.as_str(), o.total_cents))
            .collect();
        assert!(!got.is_empty());
        assert_eq!(got, expected);
    }

    #[test]
    fn ticked_offers_are_ranked_as_the_core_ranks_them() {
        let ticked = ["bolt.vw-polo".to_owned(), "miles.s".to_owned()];
        let ranking = seed().options(METRES, SECONDS, &ticked);
        let choices: Vec<cc::Choice> = ticked
            .iter()
            .map(|id| cc::Choice::Offer {
                offer: cc::OfferId(id.clone()),
            })
            .collect();
        let core = cc::options(
            &core_quotes(&cc::Catalogue::from_json(SEED).unwrap()),
            &choices,
        );
        assert_eq!(ranking.ranked.len(), 2);
        assert_eq!(
            ids(&ranking.ranked),
            core.ranked
                .iter()
                .map(|o| o.quote.offer.0.as_str())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            ids(&ranking.cheaper_not_added),
            core.cheaper_not_added
                .iter()
                .map(|q| q.offer.0.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn nothing_or_unknown_offers_rank_nothing() {
        for ticked in [vec![], vec!["gone.car".to_owned()]] {
            let ranking = seed().options(METRES, SECONDS, &ticked);
            assert!(ranking.ranked.is_empty());
            assert!(ranking.cheaper_not_added.is_empty());
        }
    }

    #[test]
    fn providers_and_offers_keep_the_catalogue_order() {
        let catalogue = cc::Catalogue::from_json(SEED).unwrap();
        let providers = seed().providers();
        assert_eq!(
            providers.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            catalogue
                .providers
                .iter()
                .map(|p| p.id.0.as_str())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            providers[0]
                .offers
                .iter()
                .map(|o| o.id.as_str())
                .collect::<Vec<_>>(),
            catalogue.providers[0]
                .offers
                .iter()
                .map(|o| o.id.0.as_str())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_server_catalogue_is_used_and_crowd_extras_are_ignored() {
        let mut catalogue: serde_json::Value = serde_json::from_str(SEED).unwrap();
        catalogue["providers"][0]["offers"][0]["cards"][0]["per_km"] = 1.into();
        let body = serde_json::json!({
            "catalogue": catalogue,
            "offers": [{"offer": "miles.s", "confirmed_on": null, "locked": false,
                        "pending": {"report": "11111111-2222-4333-8444-555555555555", "cards": []}}],
            "proposals": [{"report": "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee", "provider": "bolt",
                           "name": "Fiat 500e", "cards": []}]
        });
        let server = WebPrices::from_server(&body.to_string()).unwrap();
        let edited: cc::Catalogue = serde_json::from_value(catalogue).unwrap();
        let expected: Vec<(String, i64)> = core_quotes(&edited)
            .iter()
            .map(|q| (q.offer.0.clone(), q.total.0))
            .collect();
        let got: Vec<(String, i64)> = server
            .offers(METRES, SECONDS)
            .iter()
            .map(|o| (o.offer_id.clone(), o.total_cents))
            .collect();
        assert_eq!(got, expected, "server prices, no proposals");
    }

    #[test]
    fn invalid_input_is_an_error_not_a_panic() {
        assert!(WebPrices::from_seed("{").is_err());
        assert!(WebPrices::from_server("{}").is_err(), "no catalogue field");
        assert!(WebPrices::from_server(r#"{"catalogue": {"providers": "x"}}"#).is_err());
    }
}
