//! whichride's server: the provider catalogue with crowd prices, price reports and votes from
//! genuine app installs, a routing proxy that stores no locations, short-link expansion, and the
//! owner's emergency-switch commands. Every price and voting rule comes from `carsharing-core`.

pub mod app;
pub mod auth;
pub mod catalogue;
pub mod cli;
pub mod commands;
pub mod config;
pub mod db;
pub mod error;
pub mod http;
pub mod installs;
pub mod integrity;
pub mod state;

use std::time::{SystemTime, UNIX_EPOCH};

use carsharing_core::Day;

/// Today's UTC date from the system clock.
pub fn system_today() -> Day {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        });
    Day::from_unix_seconds(seconds)
}
