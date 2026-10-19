//! Calendar days as plain numbers, so the core needs no clock: callers pass today's date.

use serde::{Deserialize, Serialize};

/// Days since 1970-01-01 (UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Day(pub i32);

impl Day {
    pub const SECONDS_PER_DAY: i64 = 86_400;

    /// The UTC day containing this Unix time; dates beyond `i32` days saturate.
    pub fn from_unix_seconds(seconds: i64) -> Day {
        let days = seconds.div_euclid(Self::SECONDS_PER_DAY);
        Day(i32::try_from(days).unwrap_or(if days < 0 { i32::MIN } else { i32::MAX }))
    }

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

    #[test]
    fn days_from_unix_seconds() {
        assert_eq!(Day::from_unix_seconds(0), Day(0));
        assert_eq!(Day::from_unix_seconds(86_399), Day(0));
        assert_eq!(Day::from_unix_seconds(86_400), Day(1));
        // 2026-10-19T12:00:00Z
        assert_eq!(Day::from_unix_seconds(1_792_411_200), Day(20_745));
    }

    #[test]
    fn days_before_1970_round_down() {
        assert_eq!(Day::from_unix_seconds(-1), Day(-1));
        assert_eq!(Day::from_unix_seconds(-86_400), Day(-1));
        assert_eq!(Day::from_unix_seconds(-86_401), Day(-2));
    }

    #[test]
    fn extreme_seconds_saturate() {
        assert_eq!(Day::from_unix_seconds(i64::MAX), Day(i32::MAX));
        assert_eq!(Day::from_unix_seconds(i64::MIN), Day(i32::MIN));
    }
}
