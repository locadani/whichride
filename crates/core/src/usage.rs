//! A trip's distance and duration, and how providers turn them into a charge.

use serde::{Deserialize, Serialize};

use crate::money::Cents;

/// Distance and driving time of one trip, as returned by the routing service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trip {
    pub metres: u32,
    pub seconds: u32,
}

/// How a provider counts units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rounding {
    /// Every started unit is charged in full (10.2 km → 11 km).
    StartedUnit,
    /// Charged proportionally (10.2 km → 10.2 km), rounded to the cent at the end.
    Exact,
}

/// How much of `amount` (in sub-units, e.g. metres) is charged, in sub-units: every started unit of
/// `unit_size` counted in full or the exact amount, minus the first `included_units` units.
/// 10.2 km counted per started km is 11 000 metres.
pub fn billed(amount: u32, unit_size: u32, included_units: u32, rounding: Rounding) -> u64 {
    let counted = match rounding {
        Rounding::StartedUnit => u64::from(amount.div_ceil(unit_size)) * u64::from(unit_size),
        Rounding::Exact => u64::from(amount),
    };
    counted.saturating_sub(u64::from(included_units) * u64::from(unit_size))
}

/// The charge for `amount` (in sub-units, e.g. metres) at `rate` per unit of `unit_size` sub-units
/// (e.g. 1000 metres), after the first `included_units` units, which are free.
pub fn usage_charge(
    rate: Cents,
    amount: u32,
    unit_size: u32,
    included_units: u32,
    rounding: Rounding,
) -> Cents {
    let charged = billed(amount, unit_size, included_units, rounding);
    rate.scaled(charged, u64::from(unit_size))
}

pub const METRES_PER_KM: u32 = 1_000;
pub const SECONDS_PER_MINUTE: u32 = 60;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn started_unit_charges_every_started_km() {
        let charge = usage_charge(Cents(79), 10_200, METRES_PER_KM, 0, Rounding::StartedUnit);
        assert_eq!(charge, Cents(11 * 79));
    }

    #[test]
    fn started_unit_charges_exact_multiples_once() {
        let charge = usage_charge(Cents(79), 10_000, METRES_PER_KM, 0, Rounding::StartedUnit);
        assert_eq!(charge, Cents(10 * 79));
    }

    #[test]
    fn exact_charges_proportionally() {
        let charge = usage_charge(Cents(79), 10_200, METRES_PER_KM, 0, Rounding::Exact);
        assert_eq!(charge, Cents(806)); // 805.8 → 806
    }

    #[test]
    fn included_units_are_free() {
        let charge = usage_charge(
            Cents(29),
            250_000,
            METRES_PER_KM,
            200,
            Rounding::StartedUnit,
        );
        assert_eq!(charge, Cents(50 * 29));
        let charge = usage_charge(Cents(29), 250_000, METRES_PER_KM, 200, Rounding::Exact);
        assert_eq!(charge, Cents(50 * 29));
    }

    #[test]
    fn included_units_larger_than_the_trip_cost_nothing() {
        let charge = usage_charge(Cents(29), 5_000, METRES_PER_KM, 40, Rounding::StartedUnit);
        assert_eq!(charge, Cents::ZERO);
        let charge = usage_charge(Cents(29), 5_000, METRES_PER_KM, 40, Rounding::Exact);
        assert_eq!(charge, Cents::ZERO);
    }

    #[test]
    fn zero_amount_costs_nothing() {
        let charge = usage_charge(Cents(11), 0, SECONDS_PER_MINUTE, 0, Rounding::StartedUnit);
        assert_eq!(charge, Cents::ZERO);
    }

    #[test]
    fn billed_counts_every_started_unit_in_full() {
        assert_eq!(
            billed(10_200, METRES_PER_KM, 0, Rounding::StartedUnit),
            11_000
        );
    }

    #[test]
    fn billed_counts_the_exact_amount() {
        assert_eq!(billed(10_200, METRES_PER_KM, 0, Rounding::Exact), 10_200);
    }

    #[test]
    fn billed_leaves_out_included_units() {
        assert_eq!(
            billed(250_400, METRES_PER_KM, 200, Rounding::StartedUnit),
            51_000
        );
        assert_eq!(billed(250_400, METRES_PER_KM, 200, Rounding::Exact), 50_400);
        assert_eq!(billed(5_000, METRES_PER_KM, 40, Rounding::Exact), 0);
    }

    #[test]
    fn billed_does_not_overflow_on_huge_trips() {
        assert_eq!(
            billed(u32::MAX, METRES_PER_KM, 0, Rounding::StartedUnit),
            u64::from(u32::MAX.div_ceil(METRES_PER_KM)) * 1_000
        );
    }

    #[test]
    fn trip_round_trips_through_json() {
        let trip: Trip = serde_json::from_str(r#"{"metres":10200,"seconds":1500}"#).unwrap();
        assert_eq!(
            trip,
            Trip {
                metres: 10_200,
                seconds: 1_500
            }
        );
    }
}
