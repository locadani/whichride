//! Rate cards across the boundary: the app shows and edits them, and stores its overrides as the
//! core's JSON.

use carsharing_core as cc;

use crate::{DataError, invalid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CardKind {
    Standard,
    Package,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Rounding {
    StartedUnit,
    Exact,
}

/// One tariff of an offer; money in cents (see `carsharing_core::RateCard`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RateCard {
    pub label: String,
    pub kind: CardKind,
    pub base_fee: i64,
    pub per_km: i64,
    pub per_minute: i64,
    pub included_km: u32,
    pub included_minutes: u32,
    pub minimum: i64,
    pub km_rounding: Rounding,
    pub minute_rounding: Rounding,
}

fn rounding_from(r: cc::Rounding) -> Rounding {
    match r {
        cc::Rounding::StartedUnit => Rounding::StartedUnit,
        cc::Rounding::Exact => Rounding::Exact,
    }
}

fn rounding_to(r: Rounding) -> cc::Rounding {
    match r {
        Rounding::StartedUnit => cc::Rounding::StartedUnit,
        Rounding::Exact => cc::Rounding::Exact,
    }
}

impl From<&cc::RateCard> for RateCard {
    fn from(c: &cc::RateCard) -> Self {
        RateCard {
            label: c.label.clone(),
            kind: match c.kind {
                cc::CardKind::Standard => CardKind::Standard,
                cc::CardKind::Package => CardKind::Package,
            },
            base_fee: c.base_fee.0,
            per_km: c.per_km.0,
            per_minute: c.per_minute.0,
            included_km: c.included_km,
            included_minutes: c.included_minutes,
            minimum: c.minimum.0,
            km_rounding: rounding_from(c.km_rounding),
            minute_rounding: rounding_from(c.minute_rounding),
        }
    }
}

impl From<&RateCard> for cc::RateCard {
    fn from(c: &RateCard) -> Self {
        cc::RateCard {
            label: c.label.clone(),
            kind: match c.kind {
                CardKind::Standard => cc::CardKind::Standard,
                CardKind::Package => cc::CardKind::Package,
            },
            base_fee: cc::Cents(c.base_fee),
            per_km: cc::Cents(c.per_km),
            per_minute: cc::Cents(c.per_minute),
            included_km: c.included_km,
            included_minutes: c.included_minutes,
            minimum: cc::Cents(c.minimum),
            km_rounding: rounding_to(c.km_rounding),
            minute_rounding: rounding_to(c.minute_rounding),
        }
    }
}

pub(crate) fn to_core(cards: &[RateCard]) -> Vec<cc::RateCard> {
    cards.iter().map(Into::into).collect()
}

pub(crate) fn from_core(cards: &[cc::RateCard]) -> Vec<RateCard> {
    cards.iter().map(Into::into).collect()
}

/// The core's JSON of these cards (how the app stores its overrides).
#[uniffi::export]
pub fn cards_to_json(cards: Vec<RateCard>) -> String {
    serde_json::to_string(&to_core(&cards)).expect("rate cards always serialise")
}

#[uniffi::export]
pub fn cards_from_json(json: String) -> Result<Vec<RateCard>, DataError> {
    let cards: Vec<cc::RateCard> = serde_json::from_str(&json).map_err(invalid)?;
    Ok(from_core(&cards))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn standard(per_km: i64) -> RateCard {
        RateCard {
            label: "Standard".into(),
            kind: CardKind::Standard,
            base_fee: 100,
            per_km,
            per_minute: 0,
            included_km: 0,
            included_minutes: 0,
            minimum: 0,
            km_rounding: Rounding::StartedUnit,
            minute_rounding: Rounding::StartedUnit,
        }
    }

    #[test]
    fn cards_survive_a_json_round_trip_in_the_cores_format() {
        let cards = vec![standard(99)];
        let json = cards_to_json(cards.clone());
        assert!(json.contains(r#""km_rounding":"started_unit""#), "{json}");
        assert_eq!(cards_from_json(json), Ok(cards));
        assert!(cards_from_json("[{}]".into()).is_err());
    }
}
