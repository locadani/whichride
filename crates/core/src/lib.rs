//! Pricing rules for comparing the cost of one carsharing trip across providers.
//!
//! The crate is pure: no I/O, no clock, no floating point. The Android app, the website and the
//! server all call the same functions, so their numbers can never disagree.

pub mod adjustments;
pub mod catalogue;
pub mod compare;
pub mod geo;
pub mod money;
pub mod rate_card;
pub mod usage;

pub use adjustments::{Adjustments, Discount, PriceOverride, Subscription};
pub use catalogue::{Catalogue, CatalogueError, Offer, OfferId, Provider, ProviderId};
pub use compare::{Quote, compare};
pub use geo::{Coordinate, CoordinateError};
pub use money::{Cents, Percent, PercentError};
pub use rate_card::{CardKind, Charges, RateCard};
pub use usage::{Rounding, Trip};
