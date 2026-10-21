//! Bodies of `POST /v1/reports` (a price report or a new-offer proposal), checked with the
//! core's limits before the app sends them. The app sends these exact bytes and signs them
//! (decision 44), so they are built here, once.

use carsharing_core as cc;

use crate::cards::{RateCard, to_core};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum ReportError {
    #[error("a report has 1 to 5 rate cards")]
    CardCount,
    #[error("the rate cards are not valid")]
    InvalidCards,
    #[error("a value is outside what any provider charges")]
    Implausible,
    #[error("the name is empty, too long or has hidden characters")]
    BadName,
}

impl From<cc::ReportError> for ReportError {
    fn from(e: cc::ReportError) -> Self {
        match e {
            cc::ReportError::CardCount => ReportError::CardCount,
            cc::ReportError::InvalidCards => ReportError::InvalidCards,
            cc::ReportError::Implausible => ReportError::Implausible,
            cc::ReportError::BadName => ReportError::BadName,
        }
    }
}

/// The largest values a user may enter, in cents (`carsharing_core::report`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReportLimits {
    pub max_base_fee: i64,
    pub max_per_unit: i64,
    pub max_minimum: i64,
    pub max_name_chars: u32,
}

#[uniffi::export]
pub fn report_limits() -> ReportLimits {
    ReportLimits {
        max_base_fee: cc::report::MAX_BASE_FEE.0,
        max_per_unit: cc::report::MAX_PER_UNIT.0,
        max_minimum: cc::report::MAX_MINIMUM.0,
        max_name_chars: cc::report::MAX_NAME_CHARS as u32,
    }
}

#[uniffi::export]
pub fn report_body(offer_id: String, cards: Vec<RateCard>) -> Result<String, ReportError> {
    let cards = to_core(&cards);
    cc::report::check_cards(&cc::OfferId(offer_id.clone()), &cards)?;
    Ok(serde_json::json!({ "offer": offer_id, "cards": cards }).to_string())
}

#[uniffi::export]
pub fn proposal_body(
    provider_id: String,
    name: String,
    cards: Vec<RateCard>,
) -> Result<String, ReportError> {
    let name = cc::report::check_name(&name)?;
    let cards = to_core(&cards);
    cc::report::check_cards(&cc::OfferId(format!("{provider_id}.proposal")), &cards)?;
    Ok(serde_json::json!({ "provider": provider_id, "name": name, "cards": cards }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{CardKind, Rounding};

    fn card(per_km: i64) -> RateCard {
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
    fn a_report_body_is_what_the_server_reads() {
        let body = report_body("miles.s".into(), vec![card(89)]).unwrap();
        let value: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value["offer"], "miles.s");
        assert_eq!(value["cards"][0]["per_km"], 89);
        assert_eq!(value["cards"][0]["kind"], "standard");
    }

    #[test]
    fn implausible_values_are_refused_before_sending() {
        assert_eq!(
            report_body("miles.s".into(), vec![card(501)]),
            Err(ReportError::Implausible)
        );
        assert_eq!(
            report_body("miles.s".into(), vec![]),
            Err(ReportError::CardCount)
        );
    }

    #[test]
    fn a_proposal_body_has_the_trimmed_name() {
        let body = proposal_body("bolt".into(), "  Fiat 500e ".into(), vec![card(29)]).unwrap();
        let value: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value["provider"], "bolt");
        assert_eq!(value["name"], "Fiat 500e");
        assert_eq!(
            proposal_body("bolt".into(), " ".into(), vec![card(29)]),
            Err(ReportError::BadName)
        );
    }

    #[test]
    fn limits_are_the_cores() {
        assert_eq!(report_limits().max_per_unit, 500);
        assert_eq!(report_limits().max_name_chars, 60);
    }
}
