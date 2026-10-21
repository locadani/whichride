//! Limits on what users may report or propose (spec §5.1, §7.1). The server refuses anything
//! outside them and the app checks the same limits before sending, so both always agree.

use thiserror::Error;

use crate::catalogue::{OfferId, validate_cards};
use crate::money::Cents;
use crate::rate_card::RateCard;

pub const MAX_CARDS: usize = 5;
pub const MAX_LABEL_CHARS: usize = 40;
pub const MAX_NAME_CHARS: usize = 60;
/// Unlock fee of a standard tariff, or the price of a package.
pub const MAX_BASE_FEE: Cents = Cents(50_000);
/// Per km and per minute.
pub const MAX_PER_UNIT: Cents = Cents(500);
pub const MAX_MINIMUM: Cents = Cents(5_000);
pub const MAX_INCLUDED: u32 = 20_000;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReportError {
    #[error("a report has 1 to {MAX_CARDS} rate cards")]
    CardCount,
    #[error("the rate cards are not valid")]
    InvalidCards,
    #[error("a value is outside what any provider charges")]
    Implausible,
    #[error("the name is empty, too long or has hidden characters")]
    BadName,
}

/// Text shown to other users must not carry control characters or bidirectional formatting
/// (which can make a label read differently from what it is).
pub fn is_safe_text(text: &str) -> bool {
    !text.chars().any(|c| {
        c.is_control()
            || matches!(c, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
    })
}

/// The rate cards of a report or proposal: valid for the core, labelled, and believable.
pub fn check_cards(offer: &OfferId, cards: &[RateCard]) -> Result<(), ReportError> {
    if cards.is_empty() || cards.len() > MAX_CARDS {
        return Err(ReportError::CardCount);
    }
    validate_cards(offer, cards).map_err(|_| ReportError::InvalidCards)?;
    for card in cards {
        let label = card.label.trim();
        let plausible = label == card.label
            && !label.is_empty()
            && label.chars().count() <= MAX_LABEL_CHARS
            && is_safe_text(label)
            && card.base_fee <= MAX_BASE_FEE
            && card.per_km <= MAX_PER_UNIT
            && card.per_minute <= MAX_PER_UNIT
            && card.minimum <= MAX_MINIMUM
            && card.included_km <= MAX_INCLUDED
            && card.included_minutes <= MAX_INCLUDED;
        if !plausible {
            return Err(ReportError::Implausible);
        }
    }
    Ok(())
}

/// The name of a proposed offer, trimmed.
pub fn check_name(name: &str) -> Result<&str, ReportError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS || !is_safe_text(name) {
        return Err(ReportError::BadName);
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_card::fixtures::per_minute_and_km_card;

    fn offer() -> OfferId {
        OfferId("bolt.corsa".into())
    }

    #[test]
    fn safe_text_has_no_control_or_bidi_characters() {
        assert!(is_safe_text("Škoda Enyaq — 10 €/h"));
        for c in ['\u{0}', '\n', '\u{9f}', '\u{200e}', '\u{202c}', '\u{2067}'] {
            assert!(!is_safe_text(&format!("a{c}b")), "{c:?}");
        }
        assert!(
            is_safe_text("a\u{200d}b"),
            "a zero-width joiner is fine (emoji)"
        );
    }

    #[test]
    fn values_at_the_limits_are_accepted() {
        let mut card = per_minute_and_km_card();
        card.base_fee = MAX_BASE_FEE;
        card.per_km = MAX_PER_UNIT;
        card.per_minute = MAX_PER_UNIT;
        card.minimum = MAX_MINIMUM;
        card.included_km = MAX_INCLUDED;
        assert_eq!(check_cards(&offer(), &[card]), Ok(()));
    }

    #[test]
    fn values_above_the_limits_are_implausible() {
        let mut per_km = per_minute_and_km_card();
        per_km.per_km = Cents(MAX_PER_UNIT.0 + 1);
        let mut minimum = per_minute_and_km_card();
        minimum.minimum = Cents(MAX_MINIMUM.0 + 1);
        for card in [per_km, minimum] {
            assert_eq!(
                check_cards(&offer(), &[card]),
                Err(ReportError::Implausible)
            );
        }
    }

    #[test]
    fn labels_must_be_trimmed_short_and_safe() {
        for label in [
            " Standard",
            "",
            &"x".repeat(MAX_LABEL_CHARS + 1),
            "a\u{202e}b",
        ] {
            let mut card = per_minute_and_km_card();
            card.label = label.into();
            assert_eq!(
                check_cards(&offer(), &[card]),
                Err(ReportError::Implausible),
                "{label:?}"
            );
        }
    }

    #[test]
    fn card_count_and_core_validity_are_checked() {
        assert_eq!(check_cards(&offer(), &[]), Err(ReportError::CardCount));
        let six: Vec<_> = (0..6)
            .map(|i| {
                let mut card = per_minute_and_km_card();
                card.label = format!("Card {i}");
                card
            })
            .collect();
        assert_eq!(check_cards(&offer(), &six), Err(ReportError::CardCount));
        let mut negative = per_minute_and_km_card();
        negative.per_km = Cents(-1);
        assert_eq!(
            check_cards(&offer(), &[negative]),
            Err(ReportError::InvalidCards)
        );
    }

    #[test]
    fn names_are_trimmed_and_checked() {
        assert_eq!(check_name("  Fiat 500e "), Ok("Fiat 500e"));
        assert_eq!(check_name("   "), Err(ReportError::BadName));
        assert_eq!(
            check_name(&"a".repeat(MAX_NAME_CHARS + 1)),
            Err(ReportError::BadName)
        );
        assert_eq!(check_name("a\u{202e}b"), Err(ReportError::BadName));
    }
}
