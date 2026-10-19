//! Per-install hourly limits, in memory: one Cloud Run instance (spec §2), and a restart that
//! forgets the counters costs at most one extra hour's allowance. OpenRouteService's own daily
//! quota is the hard cap behind these.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::auth::Install;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Route,
    Geocode,
    Autocomplete,
    Expand,
}

impl Kind {
    /// Generous for a person, small against the free daily quotas.
    pub fn per_hour(self) -> u32 {
        match self {
            Kind::Route | Kind::Geocode | Kind::Expand => 60,
            Kind::Autocomplete => 600,
        }
    }
}

/// Counters are dropped when the map grows past this, keeping only the current hour.
const MAX_ENTRIES: usize = 100_000;

pub struct RateLimits {
    override_per_hour: Option<u32>,
    windows: Mutex<HashMap<(Uuid, Kind), (u64, u32)>>,
}

impl RateLimits {
    pub fn new() -> RateLimits {
        RateLimits {
            override_per_hour: None,
            windows: Mutex::new(HashMap::new()),
        }
    }

    /// One limit for every kind; for tests.
    pub fn with_per_hour(per_hour: u32) -> RateLimits {
        RateLimits {
            override_per_hour: Some(per_hour),
            ..RateLimits::new()
        }
    }

    pub fn allow(&self, install: Uuid, kind: Kind, now_secs: u64) -> bool {
        let hour = now_secs / 3_600;
        let limit = self.override_per_hour.unwrap_or_else(|| kind.per_hour());
        let mut windows = self
            .windows
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if windows.len() >= MAX_ENTRIES {
            windows.retain(|_, (window, _)| *window == hour);
        }
        let entry = windows.entry((install, kind)).or_insert((hour, 0));
        if entry.0 != hour {
            *entry = (hour, 0);
        }
        if entry.1 >= limit {
            return false;
        }
        entry.1 += 1;
        true
    }
}

impl Default for RateLimits {
    fn default() -> RateLimits {
        RateLimits::new()
    }
}

pub fn check(state: &AppState, install: &Install, kind: Kind) -> Result<(), ApiError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    if state.limits.allow(install.id, kind, now) {
        Ok(())
    } else {
        Err(ApiError::TooManyRequests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: u64 = 3_600;

    #[test]
    fn allows_up_to_the_limit_per_hour() {
        let limits = RateLimits::with_per_hour(2);
        let install = Uuid::new_v4();
        assert!(limits.allow(install, Kind::Route, 10 * HOUR));
        assert!(limits.allow(install, Kind::Route, 10 * HOUR + 1));
        assert!(!limits.allow(install, Kind::Route, 10 * HOUR + 2));
        assert!(limits.allow(install, Kind::Route, 11 * HOUR));
    }

    #[test]
    fn installs_and_kinds_have_separate_counters() {
        let limits = RateLimits::with_per_hour(1);
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        assert!(limits.allow(a, Kind::Route, 0));
        assert!(limits.allow(b, Kind::Route, 0));
        assert!(limits.allow(a, Kind::Geocode, 0));
        assert!(!limits.allow(a, Kind::Route, 0));
    }

    #[test]
    fn the_default_limits() {
        assert_eq!(Kind::Route.per_hour(), 60);
        assert_eq!(Kind::Autocomplete.per_hour(), 600);
    }
}
