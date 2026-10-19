//! When a crowd price report becomes the shared price (spec §7.3) and how much each install's
//! vote counts (§7.4). The server stores reports and votes and calls these functions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::day::Day;

/// Anonymous random identity of one app installation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InstallId(pub String);

/// How much one install's vote counts, from 0 to [`MAX_WEIGHT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Weight(pub u32);

pub const MAX_WEIGHT: Weight = Weight(10_000);

/// Age points of a brand-new install; one more per day, up to [`MAX_AGE_POINTS`].
const NEW_INSTALL_POINTS: u32 = 20;
const MAX_AGE_POINTS: u32 = 100;

/// What the server knows about an install's past.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallRecord {
    pub first_seen: Day,
    /// Votes on reports that have since been decided.
    pub decided_votes: u32,
    /// Of those, how many agreed with the final outcome.
    pub matching_votes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    Yes,
    No,
}

/// One install's answer about one price report, with its weight when it voted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    pub install: InstallId,
    pub answer: Answer,
    pub weight: Weight,
}

/// An install's weight: age points (20 when new, +1 a day, at most 100) times the smoothed share
/// of its past votes that matched the outcome (50 % with no history).
pub fn weight(install: &InstallRecord, today: Day) -> Weight {
    let age_points = NEW_INSTALL_POINTS
        .saturating_add(today.days_since(install.first_seen))
        .min(MAX_AGE_POINTS);
    let decided = u64::from(install.decided_votes);
    let matching = u64::from(install.matching_votes.min(install.decided_votes));
    let accuracy = (matching + 1) * 100 / (decided + 2);
    Weight(u32::try_from(u64::from(age_points) * accuracy).unwrap_or(MAX_WEIGHT.0))
}

/// Whether a price report is accepted: at least one install other than the reporter confirmed it
/// with a weight above zero, and the reporter's weight plus the weighted yes votes exceed the
/// weighted no votes. Votes are in the order they were cast; only each install's last vote
/// counts, and the reporter's own votes are ignored.
pub fn is_accepted(reporter: &InstallId, reporter_weight: Weight, votes: &[Vote]) -> bool {
    let mut last_votes: HashMap<&InstallId, &Vote> = HashMap::new();
    for vote in votes.iter().filter(|vote| &vote.install != reporter) {
        last_votes.insert(&vote.install, vote);
    }
    let mut yes = u64::from(reporter_weight.0);
    let mut no = 0;
    let mut confirmed = false;
    for vote in last_votes.values() {
        let weight = u64::from(vote.weight.0);
        match vote.answer {
            Answer::Yes => {
                yes += weight;
                confirmed |= weight > 0;
            }
            Answer::No => no += weight,
        }
    }
    confirmed && yes > no
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(first_seen: i32, decided_votes: u32, matching_votes: u32) -> InstallRecord {
        InstallRecord {
            first_seen: Day(first_seen),
            decided_votes,
            matching_votes,
        }
    }

    fn vote(install: &str, answer: Answer, weight: u32) -> Vote {
        Vote {
            install: InstallId(install.into()),
            answer,
            weight: Weight(weight),
        }
    }

    fn reporter() -> InstallId {
        InstallId("reporter".into())
    }

    #[test]
    fn a_new_install_without_history_weighs_little() {
        assert_eq!(weight(&install(100, 0, 0), Day(100)), Weight(1_000));
    }

    #[test]
    fn weight_grows_with_age_up_to_a_cap() {
        assert_eq!(weight(&install(0, 0, 0), Day(40)), Weight(3_000));
        assert_eq!(weight(&install(0, 0, 0), Day(80)), Weight(5_000));
        assert_eq!(weight(&install(0, 0, 0), Day(5_000)), Weight(5_000));
    }

    #[test]
    fn weight_follows_how_often_past_votes_matched() {
        assert_eq!(weight(&install(0, 8, 8), Day(80)), Weight(9_000));
        assert_eq!(weight(&install(0, 8, 0), Day(80)), Weight(1_000));
    }

    #[test]
    fn odd_histories_stay_in_range() {
        // More matching than decided votes: treated as all decided votes matching.
        assert_eq!(weight(&install(0, 2, 10), Day(0)), Weight(1_500));
        assert_eq!(weight(&install(0, u32::MAX, 0), Day(80)), Weight(0));
        assert!(weight(&install(i32::MIN, u32::MAX, u32::MAX), Day(i32::MAX)) <= MAX_WEIGHT);
        // First seen "in the future" (clock skew) counts as new.
        assert_eq!(weight(&install(200, 0, 0), Day(100)), Weight(1_000));
    }

    #[test]
    fn a_report_alone_is_not_accepted() {
        assert!(!is_accepted(&reporter(), Weight(5_000), &[]));
    }

    #[test]
    fn one_confirmation_from_another_install_is_enough() {
        assert!(is_accepted(
            &reporter(),
            Weight(1_000),
            &[vote("b", Answer::Yes, 1_000)]
        ));
    }

    #[test]
    fn weighted_no_votes_can_outweigh_the_yes_votes() {
        let votes = [vote("b", Answer::Yes, 1_000), vote("c", Answer::No, 5_000)];
        assert!(!is_accepted(&reporter(), Weight(1_000), &votes));
    }

    #[test]
    fn a_tie_is_not_accepted() {
        let votes = [vote("b", Answer::Yes, 1_000), vote("c", Answer::No, 2_000)];
        assert!(!is_accepted(&reporter(), Weight(1_000), &votes));
    }

    #[test]
    fn the_reporters_own_votes_are_ignored() {
        let votes = [vote("reporter", Answer::Yes, 9_000)];
        assert!(!is_accepted(&reporter(), Weight(1_000), &votes));
    }

    #[test]
    fn only_the_last_vote_of_each_install_counts() {
        let changed_to_no = [vote("b", Answer::Yes, 1_000), vote("b", Answer::No, 1_000)];
        assert!(!is_accepted(&reporter(), Weight(1_000), &changed_to_no));
        let changed_to_yes = [vote("b", Answer::No, 1_000), vote("b", Answer::Yes, 1_000)];
        assert!(is_accepted(&reporter(), Weight(1_000), &changed_to_yes));
        // Repeating a yes does not add weight: 1000 + 1000 is not more than 2500.
        let stuffed = [
            vote("b", Answer::Yes, 1_000),
            vote("b", Answer::Yes, 1_000),
            vote("b", Answer::Yes, 1_000),
            vote("c", Answer::No, 2_500),
        ];
        assert!(!is_accepted(&reporter(), Weight(1_000), &stuffed));
    }

    #[test]
    fn a_confirmation_without_weight_does_not_count() {
        assert!(!is_accepted(
            &reporter(),
            Weight(1_000),
            &[vote("b", Answer::Yes, 0)]
        ));
    }

    #[test]
    fn answers_use_lowercase_json() {
        assert_eq!(serde_json::to_string(&Answer::No).unwrap(), r#""no""#);
    }
}
