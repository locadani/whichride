//! Calendar days as plain numbers, so the core needs no clock: callers pass today's date.

use serde::{Deserialize, Serialize};

/// Days since 1970-01-01 (UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Day(pub i32);

impl Day {
    /// Whole days from `earlier` to `self`; zero if `earlier` is later (a wrong device clock).
    pub fn days_since(self, earlier: Day) -> u32 {
        u32::try_from(i64::from(self.0) - i64::from(earlier.0)).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_whole_days() {
        assert_eq!(Day(10).days_since(Day(3)), 7);
        assert_eq!(Day(3).days_since(Day(3)), 0);
    }

    #[test]
    fn a_later_date_counts_as_zero_days() {
        assert_eq!(Day(3).days_since(Day(10)), 0);
    }

    #[test]
    fn extreme_dates_do_not_overflow() {
        assert_eq!(Day(i32::MAX).days_since(Day(i32::MIN)), u32::MAX);
    }
}
