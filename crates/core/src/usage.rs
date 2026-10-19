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

/// The charge for `amount` (in sub-units, e.g. metres) at `rate` per unit of `unit_size` sub-units
/// (e.g. 1000 metres), after the first `included_units` units, which are free.
pub fn usage_charge(
    rate: Cents,
    amount: u32,
    unit_size: u32,
    included_units: u32,
    rounding: Rounding,
) -> Cents {
    match rounding {
        Rounding::StartedUnit => {
            let units = amount.div_ceil(unit_size);
            rate.times(units.saturating_sub(included_units))
        }
        Rounding::Exact => {
            let included = u64::from(included_units) * u64::from(unit_size);
            let chargeable = u64::from(amount).saturating_sub(included);
            rate.scaled(chargeable, u64::from(unit_size))
        }
    }
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
