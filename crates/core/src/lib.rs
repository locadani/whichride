//! Pricing rules for comparing the cost of one carsharing trip across providers.
//!
//! The crate is pure: no I/O, no clock, no floating point. The Android app, the website and the
//! server all call the same functions, so their numbers can never disagree.

pub mod adjustments;
pub mod catalogue;
pub mod compare;
pub mod confirmation;
pub mod day;
pub mod geo;
pub mod link;
pub mod money;
pub mod rate_card;
pub mod usage;
pub mod voting;

pub use adjustments::{Adjustments, Discount, PriceOverride, Subscription};
pub use catalogue::{Catalogue, CatalogueError, Offer, OfferId, Provider, ProviderId};
pub use compare::{Quote, compare};
pub use confirmation::{confirmed_days_ago, should_ask};
pub use day::Day;
pub use geo::{Coordinate, CoordinateError};
pub use link::{Destination, LinkError, parse_destination};
pub use money::{Cents, Percent, PercentError};
pub use rate_card::{CardKind, Charges, RateCard};
pub use usage::{Rounding, Trip};
pub use voting::{
    Answer, InstallId, InstallRecord, Vote, Weight, WeightError, is_accepted, weight,
};
