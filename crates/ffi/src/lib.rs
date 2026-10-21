//! The core's functions for the Android app, through UniFFI. The types are flat (strings,
//! integers) so the generated Kotlin stays simple; every rule lives in `carsharing-core`.
//! Personal adjustments, crowd data and report bodies are in their own modules.

use carsharing_core as cc;

uniffi::setup_scaffolding!();

mod adjustments;
mod cards;
mod prices;
mod report;

pub use adjustments::{Adjustments, Discount, PriceOverride, RefuelCredit, Subscription};
pub use cards::{CardKind, RateCard, Rounding, cards_from_json, cards_to_json};
pub use prices::{
    AppInfo, Choice, OfferInfo, Options, PROPOSAL_PREFIX, Prices, ProviderInfo, Question, Quote,
    RankedOption, day_of, proposal_prefix,
};
pub use report::{ReportError, ReportLimits, proposal_body, report_body, report_limits};

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
    /// `reason`, not `message`: Kotlin exceptions already have a `message`.
    #[error("{reason}")]
    Invalid { reason: String },
}

pub(crate) fn invalid(error: impl std::fmt::Display) -> DataError {
    DataError::Invalid {
        reason: error.to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
