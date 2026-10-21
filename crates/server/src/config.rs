//! Configuration from the environment only; `.env.example` lists every variable.

use std::fmt;
use std::net::SocketAddr;

use thiserror::Error;
use url::Url;

use crate::integrity::RegistrationKey;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("{0} is not set (see .env.example)")]
    Missing(&'static str),
    #[error("{name} is invalid: {reason}")]
    Invalid { name: &'static str, reason: String },
}

/// How Play Integrity tokens are checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityMode {
    /// Verdicts from Google for this Android package.
    Google { package: String },
    /// Only builds holding the owner's registration key may register and write.
    Key(RegistrationKey),
    /// No checks: for running the server next to an emulator. Refused on Cloud Run.
    Off,
}

/// What `serve` needs besides `DATABASE_URL`.
#[derive(Clone, PartialEq, Eq)]
pub struct ServeConfig {
    pub bind_addr: SocketAddr,
    pub ors_api_key: String,
    pub ors_base_url: Url,
    pub integrity: IntegrityMode,
}

impl fmt::Debug for ServeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServeConfig")
            .field("bind_addr", &self.bind_addr)
            .field("ors_api_key", &"<hidden>")
            .field("ors_base_url", &self.ors_base_url.as_str())
            .field("integrity", &self.integrity)
            .finish()
    }
}

impl ServeConfig {
    pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";
    pub const DEFAULT_ORS_BASE_URL: &str = "https://api.openrouteservice.org";

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<ServeConfig, ConfigError> {
        let get = |name: &str| get(name).filter(|value| !value.trim().is_empty());
        let bind_addr = get("BIND_ADDR")
            .unwrap_or_else(|| Self::DEFAULT_BIND_ADDR.to_owned())
            .trim()
            .parse()
            .map_err(|e| invalid("BIND_ADDR", e))?;
        let ors_api_key = get("ORS_API_KEY").ok_or(ConfigError::Missing("ORS_API_KEY"))?;
        let ors_base_url =
            get("ORS_BASE_URL").unwrap_or_else(|| Self::DEFAULT_ORS_BASE_URL.to_owned());
        let ors_base_url =
            Url::parse(ors_base_url.trim()).map_err(|e| invalid("ORS_BASE_URL", e))?;
        let integrity = match get("PLAY_INTEGRITY").as_deref().map(str::trim) {
            None | Some("google") => IntegrityMode::Google {
                package: get("ANDROID_PACKAGE")
                    .ok_or(ConfigError::Missing("ANDROID_PACKAGE"))?
                    .trim()
                    .to_owned(),
            },
            Some("key") => {
                let key =
                    get("REGISTRATION_KEY").ok_or(ConfigError::Missing("REGISTRATION_KEY"))?;
                IntegrityMode::Key(
                    RegistrationKey::new(&key)
                        .map_err(|reason| invalid("REGISTRATION_KEY", reason))?,
                )
            }
            Some("off") if get("K_SERVICE").is_some() => {
                return Err(invalid("PLAY_INTEGRITY", "cannot be off on Cloud Run"));
            }
            Some("off") => IntegrityMode::Off,
            Some(other) => {
                return Err(invalid(
                    "PLAY_INTEGRITY",
                    format!("expected google, key or off, got {other:?}"),
                ));
            }
        };
        Ok(ServeConfig {
            bind_addr,
            ors_api_key: ors_api_key.trim().to_owned(),
            ors_base_url,
            integrity,
        })
    }
}

/// `DATABASE_URL`, needed by every command.
pub fn database_url(get: impl Fn(&str) -> Option<String>) -> Result<String, ConfigError> {
    get("DATABASE_URL")
        .filter(|url| !url.trim().is_empty())
        .ok_or(ConfigError::Missing("DATABASE_URL"))
}

fn invalid(name: &'static str, reason: impl fmt::Display) -> ConfigError {
    ConfigError::Invalid {
        name,
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |name| map.get(name).cloned()
    }

    const MINIMAL: &[(&str, &str)] =
        &[("ORS_API_KEY", "key"), ("ANDROID_PACKAGE", "app.whichride")];

    #[test]
    fn defaults() {
        let config = ServeConfig::from_lookup(lookup(MINIMAL)).unwrap();
        assert_eq!(config.bind_addr, "127.0.0.1:8080".parse().unwrap());
        assert_eq!(
            config.ors_base_url.as_str(),
            "https://api.openrouteservice.org/"
        );
        assert_eq!(config.ors_api_key, "key");
        assert_eq!(
            config.integrity,
            IntegrityMode::Google {
                package: "app.whichride".into()
            }
        );
    }

    #[test]
    fn the_routing_key_is_required() {
        let error = ServeConfig::from_lookup(lookup(&[("ANDROID_PACKAGE", "p")])).unwrap_err();
        assert_eq!(error, ConfigError::Missing("ORS_API_KEY"));
    }

    #[test]
    fn google_integrity_needs_the_package_name() {
        let error = ServeConfig::from_lookup(lookup(&[("ORS_API_KEY", "k")])).unwrap_err();
        assert_eq!(error, ConfigError::Missing("ANDROID_PACKAGE"));
    }

    #[test]
    fn integrity_can_be_switched_off_locally() {
        let config =
            ServeConfig::from_lookup(lookup(&[("ORS_API_KEY", "k"), ("PLAY_INTEGRITY", "off")]))
                .unwrap();
        assert_eq!(config.integrity, IntegrityMode::Off);
    }

    #[test]
    fn integrity_cannot_be_switched_off_on_cloud_run() {
        let error = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "k"),
            ("PLAY_INTEGRITY", "off"),
            ("K_SERVICE", "whichride"),
        ]))
        .unwrap_err();
        assert!(matches!(
            error,
            ConfigError::Invalid {
                name: "PLAY_INTEGRITY",
                ..
            }
        ));
    }

    #[test]
    fn unknown_values_are_rejected() {
        let mut pairs = MINIMAL.to_vec();
        pairs.push(("PLAY_INTEGRITY", "maybe"));
        assert!(ServeConfig::from_lookup(lookup(&pairs)).is_err());
        let mut pairs = MINIMAL.to_vec();
        pairs.push(("BIND_ADDR", "nowhere"));
        assert!(ServeConfig::from_lookup(lookup(&pairs)).is_err());
        let mut pairs = MINIMAL.to_vec();
        pairs.push(("ORS_BASE_URL", "not a url"));
        assert!(ServeConfig::from_lookup(lookup(&pairs)).is_err());
    }

    #[test]
    fn blank_values_count_as_missing() {
        let error =
            ServeConfig::from_lookup(lookup(&[("ORS_API_KEY", "  "), ("ANDROID_PACKAGE", "p")]))
                .unwrap_err();
        assert_eq!(error, ConfigError::Missing("ORS_API_KEY"));
    }

    #[test]
    fn debug_output_hides_the_key() {
        let config = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "secret-key-123"),
            ("ANDROID_PACKAGE", "p"),
        ]))
        .unwrap();
        assert!(!format!("{config:?}").contains("secret-key-123"));
    }

    const KEY: &str = "owner-test-key-owner-test-key-owner-test-key";

    #[test]
    fn key_mode_reads_the_registration_key() {
        let config = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "k"),
            ("PLAY_INTEGRITY", "key"),
            ("REGISTRATION_KEY", KEY),
            ("K_SERVICE", "whichride"),
        ]))
        .unwrap();
        assert_eq!(
            config.integrity,
            IntegrityMode::Key(RegistrationKey::new(KEY).unwrap())
        );
    }

    #[test]
    fn key_mode_needs_a_long_enough_key() {
        let missing =
            ServeConfig::from_lookup(lookup(&[("ORS_API_KEY", "k"), ("PLAY_INTEGRITY", "key")]))
                .unwrap_err();
        assert_eq!(missing, ConfigError::Missing("REGISTRATION_KEY"));
        let short = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "k"),
            ("PLAY_INTEGRITY", "key"),
            ("REGISTRATION_KEY", "short"),
        ]))
        .unwrap_err();
        assert!(matches!(
            short,
            ConfigError::Invalid {
                name: "REGISTRATION_KEY",
                ..
            }
        ));
    }

    #[test]
    fn debug_output_hides_the_registration_key() {
        let config = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "k"),
            ("PLAY_INTEGRITY", "key"),
            ("REGISTRATION_KEY", KEY),
        ]))
        .unwrap();
        assert!(!format!("{config:?}").contains("owner"));
    }
}
