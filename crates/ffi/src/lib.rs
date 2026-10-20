//! The core's functions for the Android app, through UniFFI. The types are flat (strings,
//! integers) so the generated Kotlin stays simple; every rule lives in `carsharing-core`.

use std::collections::HashMap;
use std::sync::Arc;

use carsharing_core as cc;
use serde::Deserialize;

uniffi::setup_scaffolding!();

/// A point in integer 1e-7 degrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct Coordinate {
    pub lat_e7: i32,
    pub lon_e7: i32,
}

impl From<cc::Coordinate> for Coordinate {
    fn from(c: cc::Coordinate) -> Self {
        Coordinate {
            lat_e7: c.lat_e7,
            lon_e7: c.lon_e7,
        }
    }
}

/// Where the user wants to go, as far as a link tells (see `carsharing_core::Destination`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum Destination {
    Coordinates {
        at: Coordinate,
        label: Option<String>,
    },
    Place {
        query: String,
        near: Option<Coordinate>,
    },
    ShortLink {
        url: String,
    },
}

impl From<cc::Destination> for Destination {
    fn from(d: cc::Destination) -> Self {
        match d {
            cc::Destination::Coordinates { at, label } => Destination::Coordinates {
                at: at.into(),
                label,
            },
            cc::Destination::Place { query, near } => Destination::Place {
                query,
                near: near.map(Into::into),
            },
            cc::Destination::ShortLink { url } => Destination::ShortLink { url },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum LinkError {
    #[error("the text contains no link")]
    NoLink,
    #[error("the link is not a Google Maps link")]
    NotGoogleMaps,
    #[error("the link has no destination")]
    NoDestination,
    #[error("the link's coordinates are out of range")]
    InvalidCoordinates,
    #[error("the text is too long")]
    TooLong,
}

impl From<cc::LinkError> for LinkError {
    fn from(e: cc::LinkError) -> Self {
        match e {
            cc::LinkError::NoLink => LinkError::NoLink,
            cc::LinkError::NotGoogleMaps => LinkError::NotGoogleMaps,
            cc::LinkError::NoDestination => LinkError::NoDestination,
            cc::LinkError::InvalidCoordinates => LinkError::InvalidCoordinates,
            cc::LinkError::TooLong => LinkError::TooLong,
        }
    }
}

/// Data from the server or the APK that the core refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum DataError {
    #[error("{message}")]
    Invalid { message: String },
}

fn invalid(error: impl std::fmt::Display) -> DataError {
    DataError::Invalid {
        message: error.to_string(),
    }
}

#[uniffi::export]
pub fn parse_destination(text: String) -> Result<Destination, LinkError> {
    Ok(cc::parse_destination(&text)?.into())
}

#[uniffi::export]
pub fn shared_label(text: String) -> Option<String> {
    cc::link::shared_label(&text)
}

/// The body of the server's `POST /v1/expand`.
#[uniffi::export]
pub fn destination_from_json(json: String) -> Result<Destination, DataError> {
    let destination: cc::Destination = serde_json::from_str(&json).map_err(invalid)?;
    Ok(destination.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct Trip {
    pub metres: u32,
    pub seconds: u32,
}

impl From<Trip> for cc::Trip {
    fn from(t: Trip) -> Self {
        cc::Trip {
            metres: t.metres,
            seconds: t.seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OfferInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub android_package: String,
    pub offers: Vec<OfferInfo>,
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

/// The fields of `GET /v1/catalogue` the app uses in 5a (pending reports and proposals are 5b).
#[derive(Deserialize)]
struct ServerCatalogue {
    catalogue: cc::Catalogue,
    offers: Vec<ServerOfferStatus>,
}

#[derive(Deserialize)]
struct ServerOfferStatus {
    offer: cc::OfferId,
    confirmed_on: Option<cc::Day>,
}

/// A validated catalogue and when each price was last confirmed.
#[derive(uniffi::Object)]
pub struct Prices {
    catalogue: cc::Catalogue,
    confirmed_on: HashMap<cc::OfferId, cc::Day>,
}

#[uniffi::export]
impl Prices {
    /// The body of the server's `GET /v1/catalogue`.
    #[uniffi::constructor]
    pub fn from_server_response(json: String) -> Result<Arc<Prices>, DataError> {
        let response: ServerCatalogue = serde_json::from_str(&json).map_err(invalid)?;
        response.catalogue.validate().map_err(invalid)?;
        let confirmed_on = response
            .offers
            .into_iter()
            .filter_map(|status| Some((status.offer, status.confirmed_on?)))
            .collect();
        Ok(Arc::new(Prices {
            catalogue: response.catalogue,
            confirmed_on,
        }))
    }

    /// The seed catalogue shipped in the APK (`crates/server/catalogue.json`); nothing is confirmed.
    #[uniffi::constructor]
    pub fn from_seed(json: String) -> Result<Arc<Prices>, DataError> {
        let catalogue = cc::Catalogue::from_json(&json).map_err(invalid)?;
        Ok(Arc::new(Prices {
            catalogue,
            confirmed_on: HashMap::new(),
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
            })
            .collect()
    }

    /// Every offer's best price, cheapest first. Personal adjustments arrive in milestone 5b.
    pub fn quotes(&self, trip: Trip, today_unix_seconds: i64) -> Vec<Quote> {
        let today = cc::Day::from_unix_seconds(today_unix_seconds);
        self.core_quotes(trip)
            .iter()
            .map(|q| self.quote(q, today))
            .collect()
    }

    /// The user's added cars ranked, and the cheaper ones they did not add (spec §4.1).
    pub fn options(&self, trip: Trip, choices: Vec<Choice>, today_unix_seconds: i64) -> Options {
        let today = cc::Day::from_unix_seconds(today_unix_seconds);
        let quotes = self.core_quotes(trip);
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
}

impl Prices {
    fn core_quotes(&self, trip: Trip) -> Vec<cc::Quote> {
        cc::compare(&self.catalogue, &trip.into(), &cc::Adjustments::default())
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &str = include_str!("../../server/catalogue.json");
    /// 2026-10-20 12:00 UTC = day 20_746.
    const TODAY: i64 = 1_792_497_600;

    fn server_response() -> String {
        let catalogue: serde_json::Value = serde_json::from_str(SEED).unwrap();
        serde_json::json!({
            "catalogue": catalogue,
            "offers": [
                {"offer": "miles.s", "confirmed_on": 20_744, "locked": false, "pending": null},
                {"offer": "bolt.vw-polo", "confirmed_on": null, "locked": false, "pending": null}
            ],
            "proposals": []
        })
        .to_string()
    }

    const TRIP: Trip = Trip {
        metres: 7_400,
        seconds: 22 * 60,
    };

    #[test]
    fn parses_a_coordinate_link() {
        let destination = parse_destination(
            "https://www.google.com/maps/place/Brandenburger+Tor/@52.5162746,13.3777041,17z".into(),
        );
        assert!(matches!(
            destination,
            Ok(Destination::Coordinates { .. }) | Ok(Destination::Place { .. })
        ));
    }

    #[test]
    fn link_errors_cross_the_boundary() {
        assert_eq!(parse_destination("hello".into()), Err(LinkError::NoLink));
        assert_eq!(
            parse_destination("https://example.com".into()),
            Err(LinkError::NotGoogleMaps)
        );
    }

    #[test]
    fn short_links_stay_short_links() {
        assert_eq!(
            parse_destination("Markthalle Neun https://maps.app.goo.gl/x7Qa2Lm".into()),
            Ok(Destination::ShortLink {
                url: "https://maps.app.goo.gl/x7Qa2Lm".into()
            })
        );
        assert_eq!(
            shared_label("Markthalle Neun\nhttps://maps.app.goo.gl/x7Qa2Lm".into()).as_deref(),
            Some("Markthalle Neun")
        );
    }

    #[test]
    fn reads_the_servers_expanded_destination() {
        let json = r#"{"type":"coordinates","at":{"lat_e7":525000000,"lon_e7":134000000},"label":"Mauerpark"}"#;
        assert_eq!(
            destination_from_json(json.into()),
            Ok(Destination::Coordinates {
                at: Coordinate {
                    lat_e7: 525_000_000,
                    lon_e7: 134_000_000
                },
                label: Some("Mauerpark".into())
            })
        );
        assert!(destination_from_json("{}".into()).is_err());
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
    }

    #[test]
    fn seed_quotes_have_no_confirmation_date() {
        let prices = Prices::from_seed(SEED.into()).unwrap();
        let quotes = prices.quotes(TRIP, TODAY);
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
        let quotes = prices.quotes(TRIP, TODAY);
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
