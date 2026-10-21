//! One way an offer can be paid for: the standard tariff or a package.

use serde::{Deserialize, Serialize};

use crate::money::Cents;
use crate::usage::{METRES_PER_KM, Rounding, SECONDS_PER_MINUTE, Trip, billed, usage_charge};

/// Whether a rate card is the pay-as-you-go tariff or a bundle bought before the trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    Standard,
    Package,
}

/// The prices of one tariff. Every provider's pricing is a subset of these components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateCard {
    /// Shown to the user, e.g. "Standard" or "3 h / 40 km".
    pub label: String,
    pub kind: CardKind,
    /// Unlock fee for a standard tariff, the package price for a package.
    pub base_fee: Cents,
    pub per_km: Cents,
    pub per_minute: Cents,
    #[serde(default)]
    pub included_km: u32,
    #[serde(default)]
    pub included_minutes: u32,
    /// Applied last, after personal adjustments (spec §6).
    #[serde(default)]
    pub minimum: Cents,
    pub km_rounding: Rounding,
    pub minute_rounding: Rounding,
}

/// The parts of a trip's price before personal adjustments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Charges {
    pub base_fee: Cents,
    pub distance: Cents,
    pub time: Cents,
    /// The metres `distance` is charged for, after rounding and included km.
    pub billed_metres: u64,
    /// The seconds `time` is charged for, after rounding and included minutes.
    pub billed_seconds: u64,
}

impl Charges {
    pub fn total(&self) -> Cents {
        self.base_fee + self.distance + self.time
    }
}

impl RateCard {
    /// What this tariff charges for the trip, before adjustments and before the minimum.
    pub fn charges(&self, trip: &Trip) -> Charges {
        let billed_metres = billed(
            trip.metres,
            METRES_PER_KM,
            self.included_km,
            self.km_rounding,
        );
        let billed_seconds = billed(
            trip.seconds,
            SECONDS_PER_MINUTE,
            self.included_minutes,
            self.minute_rounding,
        );
        Charges {
            base_fee: self.base_fee,
            distance: usage_charge(
                self.per_km,
                trip.metres,
                METRES_PER_KM,
                self.included_km,
                self.km_rounding,
            ),
            time: usage_charge(
                self.per_minute,
                trip.seconds,
                SECONDS_PER_MINUTE,
                self.included_minutes,
                self.minute_rounding,
            ),
            billed_metres,
            billed_seconds,
        }
    }
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    /// MILES-like: unlock fee + every started km.
    pub fn per_km_card() -> RateCard {
        RateCard {
            label: "Standard".into(),
            kind: CardKind::Standard,
            base_fee: Cents(100),
            per_km: Cents(79),
            per_minute: Cents::ZERO,
            included_km: 0,
            included_minutes: 0,
            minimum: Cents::ZERO,
            km_rounding: Rounding::StartedUnit,
            minute_rounding: Rounding::StartedUnit,
        }
    }

    /// Bolt-like: per started minute + per started km, with a minimum.
    pub fn per_minute_and_km_card() -> RateCard {
        RateCard {
            label: "Standard".into(),
            kind: CardKind::Standard,
            base_fee: Cents::ZERO,
            per_km: Cents(18),
            per_minute: Cents(11),
            included_km: 0,
            included_minutes: 0,
            minimum: Cents(250),
            km_rounding: Rounding::StartedUnit,
            minute_rounding: Rounding::StartedUnit,
        }
    }

    /// A 3-hour package with 40 km included.
    pub fn package_card() -> RateCard {
        RateCard {
            label: "3 h / 40 km".into(),
            kind: CardKind::Package,
            base_fee: Cents(2_900),
            per_km: Cents(39),
            per_minute: Cents(29),
            included_km: 40,
            included_minutes: 180,
            minimum: Cents::ZERO,
            km_rounding: Rounding::StartedUnit,
            minute_rounding: Rounding::StartedUnit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    const TRIP: Trip = Trip {
        metres: 10_200,
        seconds: 25 * 60,
    };

    #[test]
    fn per_km_card_charges_unlock_and_started_km() {
        let charges = per_km_card().charges(&TRIP);
        assert_eq!(
            charges,
            Charges {
                base_fee: Cents(100),
                distance: Cents(869),
                time: Cents::ZERO,
                billed_metres: 11_000,
                billed_seconds: 25 * 60,
            }
        );
        assert_eq!(charges.total(), Cents(969));
    }

    #[test]
    fn per_minute_and_km_card_charges_both() {
        let charges = per_minute_and_km_card().charges(&TRIP);
        assert_eq!(charges.total(), Cents(11 * 18 + 25 * 11));
    }

    #[test]
    fn minimum_is_not_applied_by_charges() {
        let short = Trip {
            metres: 500,
            seconds: 120,
        };
        assert_eq!(
            per_minute_and_km_card().charges(&short).total(),
            Cents(18 + 22)
        );
    }

    #[test]
    fn package_charges_only_the_price_within_its_allowance() {
        assert_eq!(package_card().charges(&TRIP).total(), Cents(2_900));
    }

    #[test]
    fn package_charges_overage_beyond_its_allowance() {
        let long = Trip {
            metres: 45_000,
            seconds: 200 * 60,
        };
        let charges = package_card().charges(&long);
        assert_eq!(charges.distance, Cents(5 * 39));
        assert_eq!(charges.time, Cents(20 * 29));
    }

    #[test]
    fn rate_card_reads_optional_fields_as_zero() {
        let json = r#"{
            "label": "Standard", "kind": "standard",
            "base_fee": 100, "per_km": 79, "per_minute": 0,
            "km_rounding": "started_unit", "minute_rounding": "exact"
        }"#;
        let card: RateCard = serde_json::from_str(json).unwrap();
        assert_eq!(card.included_km, 0);
        assert_eq!(card.minimum, Cents::ZERO);
        assert_eq!(card.minute_rounding, Rounding::Exact);
    }
}
