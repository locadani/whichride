//! Money in integer euro cents and percentages in basis points: no floating point anywhere.

use std::iter::Sum;
use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// An amount of money in euro cents.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Cents(pub i64);

impl Cents {
    pub const ZERO: Cents = Cents(0);

    /// This price multiplied by a number of units, saturating at the `i64` limits.
    pub fn times(self, units: u32) -> Cents {
        Cents(self.0.saturating_mul(i64::from(units)))
    }

    /// `self × numerator ÷ denominator`, rounded half up to a whole cent, saturating at the `i64`
    /// limits.
    pub fn scaled(self, numerator: u64, denominator: u64) -> Cents {
        assert!(denominator > 0, "denominator must be positive");
        let product = i128::from(self.0).saturating_mul(i128::from(numerator));
        let denominator = i128::from(denominator);
        let rounded = product
            .saturating_mul(2)
            .saturating_add(denominator)
            .div_euclid(2 * denominator);
        Cents(rounded.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64)
    }

    /// This amount, or zero if it is negative.
    pub fn at_least_zero(self) -> Cents {
        self.max(Cents::ZERO)
    }
}

impl Add for Cents {
    type Output = Cents;
    fn add(self, rhs: Cents) -> Cents {
        Cents(self.0.saturating_add(rhs.0))
    }
}

impl Sub for Cents {
    type Output = Cents;
    fn sub(self, rhs: Cents) -> Cents {
        Cents(self.0.saturating_sub(rhs.0))
    }
}

impl Sum for Cents {
    fn sum<I: Iterator<Item = Cents>>(iter: I) -> Cents {
        iter.fold(Cents::ZERO, Add::add)
    }
}

/// A percentage in basis points: 1 % = 100, 100 % = 10 000.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(try_from = "u16", into = "u16")]
pub struct Percent(u16);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("a percentage must be between 0 and 10000 basis points, got {0}")]
pub struct PercentError(pub u16);

impl Percent {
    pub const ZERO: Percent = Percent(0);

    pub fn from_basis_points(basis_points: u16) -> Result<Percent, PercentError> {
        if basis_points > 10_000 {
            return Err(PercentError(basis_points));
        }
        Ok(Percent(basis_points))
    }

    pub fn basis_points(self) -> u16 {
        self.0
    }

    /// The amount left after taking this percentage off, rounded half up to a whole cent.
    pub fn off(self, amount: Cents) -> Cents {
        amount.scaled(u64::from(10_000 - self.0), 10_000)
    }
}

impl TryFrom<u16> for Percent {
    type Error = PercentError;
    fn try_from(basis_points: u16) -> Result<Percent, PercentError> {
        Percent::from_basis_points(basis_points)
    }
}

impl From<Percent> for u16 {
    fn from(percent: Percent) -> u16 {
        percent.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_multiplies_by_units() {
        assert_eq!(Cents(79).times(11), Cents(869));
    }

    #[test]
    fn scaled_rounds_half_up() {
        assert_eq!(Cents(79).scaled(500, 1000), Cents(40)); // 39.5 → 40
        assert_eq!(Cents(79).scaled(499, 1000), Cents(39)); // 39.421 → 39
        assert_eq!(Cents(10).scaled(1, 3), Cents(3)); // 3.33 → 3
    }

    #[test]
    fn scaled_does_not_overflow_for_large_trips() {
        // 3 days in seconds at 50 cents a minute
        assert_eq!(Cents(50).scaled(259_200, 60), Cents(216_000));
    }

    #[test]
    fn at_least_zero_clamps_negative_amounts() {
        assert_eq!((Cents(300) - Cents(500)).at_least_zero(), Cents::ZERO);
        assert_eq!(Cents(300).at_least_zero(), Cents(300));
    }

    #[test]
    fn sum_adds_all_amounts() {
        let total: Cents = [Cents(100), Cents(250), Cents(5)].into_iter().sum();
        assert_eq!(total, Cents(355));
    }

    #[test]
    fn percent_rejects_more_than_one_hundred_percent() {
        assert_eq!(
            Percent::from_basis_points(10_001),
            Err(PercentError(10_001))
        );
        assert!(Percent::from_basis_points(10_000).is_ok());
    }

    #[test]
    fn percent_off_leaves_the_rest_rounded_half_up() {
        let twenty = Percent::from_basis_points(2_000).unwrap();
        assert_eq!(twenty.off(Cents(969)), Cents(775)); // 775.2 → 775
        assert_eq!(twenty.off(Cents(1_000)), Cents(800));
        let fifteen = Percent::from_basis_points(1_500).unwrap();
        assert_eq!(fifteen.off(Cents(3)), Cents(3)); // 2.55 → 3
    }

    #[test]
    fn percent_deserialises_only_valid_values() {
        assert_eq!(
            serde_json::from_str::<Percent>("2500")
                .unwrap()
                .basis_points(),
            2_500
        );
        assert!(serde_json::from_str::<Percent>("12000").is_err());
    }

    #[test]
    fn arithmetic_saturates_instead_of_overflowing() {
        assert_eq!(Cents(i64::MAX).times(2), Cents(i64::MAX));
        assert_eq!(Cents(i64::MAX) + Cents(1), Cents(i64::MAX));
        assert_eq!(Cents(i64::MIN) - Cents(1), Cents(i64::MIN));
        assert_eq!(Cents(i64::MAX).scaled(u64::MAX, 1), Cents(i64::MAX));
    }
}
