//! whichride's server: the provider catalogue with crowd prices, price reports and votes from
//! genuine app installs, a routing proxy that stores no locations, short-link expansion, and the
//! owner's emergency-switch commands. Every price and voting rule comes from `carsharing-core`.

pub mod app;
pub mod auth;
pub mod catalogue;
pub mod cli;
pub mod commands;
pub mod config;
pub mod crowd;
pub mod db;
pub mod error;
pub mod expand;
pub mod http;
pub mod installs;
pub mod integrity;
pub mod limits;
pub mod ors;
pub mod owner;
pub mod routing;
pub mod state;

use std::time::{SystemTime, UNIX_EPOCH};

use carsharing_core::Day;

/// The log filter: `RUST_LOG` (or `info` when unset or invalid), always with the HTTP client
/// libraries silenced, as their debug output can contain URLs with coordinates or links.
pub fn log_filter(rust_log: Option<&str>) -> tracing_subscriber::EnvFilter {
    let filter = rust_log
        .and_then(|directives| tracing_subscriber::EnvFilter::try_new(directives).ok())
        .unwrap_or_else(|| tracing_subscriber::EnvFilter::new("info"));
    ["reqwest=off", "hyper=off", "hyper_util=off"]
        .into_iter()
        .fold(filter, |filter, directive| {
            filter.add_directive(directive.parse().expect("a valid directive"))
        })
}

/// Today's UTC date from the system clock.
pub fn system_today() -> Day {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        });
    Day::from_unix_seconds(seconds)
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);

    impl Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// What is logged with this filter when every library logs at every level.
    fn logged(rust_log: Option<&str>) -> String {
        let sink = Sink::default();
        let writer = sink.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(log_filter(rust_log))
            .without_time()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            tracing::error!(target: "reqwest::connect", "reqwest error");
            tracing::trace!(target: "reqwest", "reqwest trace");
            tracing::error!(target: "hyper", "hyper error");
            tracing::error!(target: "hyper_util::client::legacy", "hyper_util error");
            tracing::info!(target: "whichride_server", "server info");
            tracing::debug!(target: "whichride_server", "server debug");
        });
        String::from_utf8(sink.0.lock().unwrap().clone()).unwrap()
    }

    #[test]
    fn http_client_logs_are_always_off() {
        for rust_log in [
            None,
            Some("trace"),
            Some("info,reqwest=trace,hyper=trace,hyper_util=trace"),
            Some("not a filter=!!"),
        ] {
            let logs = logged(rust_log);
            for library in ["reqwest", "hyper error", "hyper_util"] {
                assert!(!logs.contains(library), "{rust_log:?}: {logs}");
            }
            assert!(logs.contains("server info"), "{rust_log:?}: {logs}");
        }
        assert!(!logged(None).contains("server debug"));
        assert!(logged(Some("debug")).contains("server debug"));
    }
}
