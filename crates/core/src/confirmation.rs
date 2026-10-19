//! When the app asks the user to confirm a price (spec §7.2), and how fresh a price is (§7.5).

use crate::day::Day;

/// The app asks at most once in this many days per install.
pub const ASK_INTERVAL_DAYS: u32 = 7;

/// A price not confirmed for this many days is worth asking about.
pub const STALE_AFTER_DAYS: u32 = 14;

/// Whether to show the one-question confirmation when the user comes back from a provider's app.
///
/// A `last_asked` date later than `today` (a wrong device clock) does not block asking.
pub fn should_ask(
    today: Day,
    last_asked: Option<Day>,
    pending_report: bool,
    last_confirmed: Option<Day>,
) -> bool {
    let asked_recently =
        last_asked.is_some_and(|day| day <= today && today.days_since(day) < ASK_INTERVAL_DAYS);
    let stale = last_confirmed.is_none_or(|day| today.days_since(day) >= STALE_AFTER_DAYS);
    !asked_recently && (pending_report || stale)
}

/// Whole days since the price was last confirmed, for the "confirmed 2 days ago" label;
/// `None` if it never was.
pub fn confirmed_days_ago(today: Day, last_confirmed: Option<Day>) -> Option<u32> {
    last_confirmed.map(|day| today.days_since(day))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: Day = Day(20_000);

    fn days_ago(days: u32) -> Option<Day> {
        Some(Day(TODAY.0 - i32::try_from(days).unwrap()))
    }

    fn days_ahead(days: i32) -> Option<Day> {
        Some(Day(TODAY.0 + days))
    }

    #[test]
    fn asks_about_a_pending_report_when_never_asked() {
        assert!(should_ask(TODAY, None, true, days_ago(1)));
    }

    #[test]
    fn asks_about_a_price_never_confirmed() {
        assert!(should_ask(TODAY, None, false, None));
    }

    #[test]
    fn asks_at_most_once_per_interval() {
        assert!(!should_ask(TODAY, days_ago(0), true, None));
        assert!(!should_ask(
            TODAY,
            days_ago(ASK_INTERVAL_DAYS - 1),
            true,
            None
        ));
        assert!(should_ask(TODAY, days_ago(ASK_INTERVAL_DAYS), true, None));
    }

    #[test]
    fn does_not_ask_about_a_recently_confirmed_price() {
        assert!(!should_ask(TODAY, None, false, days_ago(0)));
        assert!(!should_ask(
            TODAY,
            None,
            false,
            days_ago(STALE_AFTER_DAYS - 1)
        ));
        assert!(should_ask(TODAY, None, false, days_ago(STALE_AFTER_DAYS)));
    }

    #[test]
    fn a_last_asked_date_in_the_future_does_not_block_asking() {
        assert!(should_ask(TODAY, days_ahead(400), true, None));
    }

    #[test]
    fn freshness_counts_days_since_confirmation() {
        assert_eq!(confirmed_days_ago(TODAY, days_ago(2)), Some(2));
        assert_eq!(confirmed_days_ago(TODAY, None), None);
        // Confirmed "in the future" (clock skew) shows as today.
        assert_eq!(confirmed_days_ago(TODAY, days_ahead(3)), Some(0));
    }
}
