//! Play Integrity (spec §7.4): a write is accepted only from a genuine app on a genuine device,
//! and only for the exact request the app asked Google to vouch for.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::Mutex;
use url::Url;

use crate::error::ApiError;

pub const TOKEN_HEADER: &str = "x-integrity-token";

/// Real tokens are a few kilobytes; anything longer is not sent to Google.
pub const MAX_TOKEN_BYTES: usize = 8 * 1024;

/// A verdict older than this is refused; the app asks for a token right before each write.
const MAX_AGE_MILLIS: i64 = 10 * 60 * 1000;
/// Tolerated difference between Google's clock and ours.
const MAX_FUTURE_MILLIS: i64 = 60 * 1000;

const SCOPE: &str = "https://www.googleapis.com/auth/playintegrity";

/// Lowercase hex SHA-256 of `"{method} {path}\n"` followed by the body. The app sets it as the
/// token's `requestHash`, so a token is good for this one request only.
pub fn request_hash(method: &str, path: &str, body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(method.as_bytes());
    hasher.update(b" ");
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(body);
    hex::encode(hasher.finalize())
}

/// The parts of Google's decoded token this server checks.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verdict {
    pub request_details: RequestDetails,
    pub app_integrity: AppIntegrity,
    pub device_integrity: DeviceIntegrity,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestDetails {
    pub request_package_name: String,
    #[serde(default)]
    pub request_hash: Option<String>,
    /// Google sends 64-bit integers as JSON strings.
    pub timestamp_millis: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppIntegrity {
    pub app_recognition_verdict: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIntegrity {
    #[serde(default)]
    pub device_recognition_verdict: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IntegrityError {
    #[error("no integrity token")]
    Missing,
    #[error("integrity verdict refused: {0}")]
    Refused(&'static str),
    #[error("the integrity service is unavailable")]
    Unavailable,
}

impl From<IntegrityError> for ApiError {
    fn from(error: IntegrityError) -> ApiError {
        match error {
            IntegrityError::Missing | IntegrityError::Refused(_) => {
                tracing::info!(%error, "integrity check failed");
                ApiError::IntegrityFailed
            }
            IntegrityError::Unavailable => {
                tracing::warn!(%error, "integrity check unavailable");
                ApiError::IntegrityUnavailable
            }
        }
    }
}

/// Checks a decoded verdict: our package, recognised by Play, a device with integrity, this
/// request, and recent.
pub fn check(
    verdict: &Verdict,
    package: &str,
    expected_hash: &str,
    now_millis: i64,
) -> Result<(), IntegrityError> {
    let details = &verdict.request_details;
    if details.request_package_name != package {
        return Err(IntegrityError::Refused("package"));
    }
    if verdict.app_integrity.app_recognition_verdict != "PLAY_RECOGNIZED" {
        return Err(IntegrityError::Refused("app"));
    }
    let device = &verdict.device_integrity.device_recognition_verdict;
    if !device.iter().any(|label| label == "MEETS_DEVICE_INTEGRITY") {
        return Err(IntegrityError::Refused("device"));
    }
    if details.request_hash.as_deref() != Some(expected_hash) {
        return Err(IntegrityError::Refused("request hash"));
    }
    let issued: i64 = details
        .timestamp_millis
        .parse()
        .map_err(|_| IntegrityError::Refused("stale"))?;
    let age = now_millis.saturating_sub(issued);
    if !(-MAX_FUTURE_MILLIS..=MAX_AGE_MILLIS).contains(&age) {
        return Err(IntegrityError::Refused("stale"));
    }
    Ok(())
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
        })
}

// One value per process, held in an `Arc`: the size difference costs nothing.
#[allow(clippy::large_enum_variant)]
pub enum Integrity {
    Google(GoogleIntegrity),
    /// Local development only; the configuration refuses it on Cloud Run.
    Off,
}

impl Integrity {
    pub async fn verify(
        &self,
        token: Option<&str>,
        expected_hash: &str,
    ) -> Result<(), IntegrityError> {
        let Integrity::Google(google) = self else {
            return Ok(());
        };
        let token = token
            .filter(|token| !token.is_empty() && token.len() <= MAX_TOKEN_BYTES)
            .ok_or(IntegrityError::Missing)?;
        let verdict = google.decode(token).await?;
        check(&verdict, &google.package, expected_hash, now_millis())
    }
}

/// Decodes tokens with Google, authenticated as the Cloud Run service account (whose access
/// token comes from the metadata server; no key file exists).
pub struct GoogleIntegrity {
    http: reqwest::Client,
    package: String,
    api_base: Url,
    metadata_base: Url,
    access: Mutex<Option<(String, Instant)>>,
}

#[derive(Deserialize)]
struct AccessToken {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecodeResponse {
    token_payload_external: Verdict,
}

impl GoogleIntegrity {
    pub fn new(package: String) -> GoogleIntegrity {
        GoogleIntegrity::with_endpoints(
            package,
            Url::parse("https://playintegrity.googleapis.com/").expect("valid URL"),
            Url::parse("http://metadata.google.internal/").expect("valid URL"),
        )
    }

    /// With other base URLs, for tests.
    pub fn with_endpoints(package: String, api_base: Url, metadata_base: Url) -> GoogleIntegrity {
        GoogleIntegrity {
            http: crate::http::client(Duration::from_secs(5)),
            package,
            api_base,
            metadata_base,
            access: Mutex::new(None),
        }
    }

    async fn access_token(&self) -> Result<String, IntegrityError> {
        let mut cached = self.access.lock().await;
        if let Some((token, expires)) = cached.as_ref()
            && *expires > Instant::now() + Duration::from_secs(60)
        {
            return Ok(token.clone());
        }
        let url = self
            .metadata_base
            .join("computeMetadata/v1/instance/service-accounts/default/token")
            .map_err(|_| IntegrityError::Unavailable)?;
        let fresh: AccessToken = self
            .http
            .get(url)
            .query(&[("scopes", SCOPE)])
            .header("Metadata-Flavor", "Google")
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|_| IntegrityError::Unavailable)?
            .json()
            .await
            .map_err(|_| IntegrityError::Unavailable)?;
        let expires = Instant::now() + Duration::from_secs(fresh.expires_in);
        *cached = Some((fresh.access_token.clone(), expires));
        Ok(fresh.access_token)
    }

    async fn decode(&self, token: &str) -> Result<Verdict, IntegrityError> {
        let access = self.access_token().await?;
        let url = self
            .api_base
            .join(&format!("v1/{}:decodeIntegrityToken", self.package))
            .map_err(|_| IntegrityError::Unavailable)?;
        let response = self
            .http
            .post(url)
            .bearer_auth(access)
            .json(&serde_json::json!({ "integrityToken": token }))
            .send()
            .await
            .map_err(|_| IntegrityError::Unavailable)?;
        let status = response.status();
        match status.as_u16() {
            // The token itself is bad.
            400 | 404 => return Err(IntegrityError::Refused("token")),
            // Our credentials were rejected: forget the access token, it is not the app's fault.
            401 | 403 => {
                *self.access.lock().await = None;
                return Err(IntegrityError::Unavailable);
            }
            _ if !status.is_success() => return Err(IntegrityError::Unavailable),
            _ => {}
        }
        let decoded: DecodeResponse = response
            .json()
            .await
            .map_err(|_| IntegrityError::Unavailable)?;
        Ok(decoded.token_payload_external)
    }
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const PACKAGE: &str = "app.whichride";
    const NOW: i64 = 1_792_411_200_000;

    fn verdict_json(hash: &str, timestamp: i64) -> serde_json::Value {
        serde_json::json!({
            "requestDetails": {
                "requestPackageName": PACKAGE,
                "requestHash": hash,
                "timestampMillis": timestamp.to_string()
            },
            "appIntegrity": {
                "appRecognitionVerdict": "PLAY_RECOGNIZED",
                "packageName": PACKAGE,
                "certificateSha256Digest": ["abc"],
                "versionCode": "1"
            },
            "deviceIntegrity": {
                "deviceRecognitionVerdict": ["MEETS_DEVICE_INTEGRITY"]
            },
            "accountDetails": { "appLicensingVerdict": "LICENSED" }
        })
    }

    fn verdict(hash: &str, timestamp: i64) -> Verdict {
        serde_json::from_value(verdict_json(hash, timestamp)).unwrap()
    }

    #[test]
    fn the_request_hash_covers_method_path_and_body() {
        let hash = request_hash("POST", "/v1/reports", br#"{"a":1}"#);
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, request_hash("POST", "/v1/reports", br#"{"a":1}"#));
        assert_ne!(hash, request_hash("POST", "/v1/reports", br#"{"a":2}"#));
        assert_ne!(hash, request_hash("POST", "/v1/installs", br#"{"a":1}"#));
        // Known value, so the Android implementation can be checked against it.
        assert_eq!(
            request_hash("POST", "/v1/installs", b"{}"),
            "9bc58881cd740b3aa0d48e5698ed16bb88cd9949784c69f48fd4f918787a1099"
        );
    }

    #[test]
    fn a_good_verdict_passes() {
        assert_eq!(check(&verdict("h", NOW), PACKAGE, "h", NOW), Ok(()));
        assert_eq!(
            check(&verdict("h", NOW - 9 * 60_000), PACKAGE, "h", NOW),
            Ok(())
        );
    }

    #[test]
    fn a_different_request_is_refused() {
        assert_eq!(
            check(&verdict("other", NOW), PACKAGE, "h", NOW),
            Err(IntegrityError::Refused("request hash"))
        );
    }

    #[test]
    fn another_package_is_refused() {
        assert_eq!(
            check(&verdict("h", NOW), "com.example.fake", "h", NOW),
            Err(IntegrityError::Refused("package"))
        );
    }

    #[test]
    fn an_app_not_from_play_is_refused() {
        let mut json = verdict_json("h", NOW);
        json["appIntegrity"]["appRecognitionVerdict"] = "UNRECOGNIZED_VERSION".into();
        let verdict: Verdict = serde_json::from_value(json).unwrap();
        assert_eq!(
            check(&verdict, PACKAGE, "h", NOW),
            Err(IntegrityError::Refused("app"))
        );
    }

    #[test]
    fn a_device_without_integrity_is_refused() {
        let mut json = verdict_json("h", NOW);
        json["deviceIntegrity"] = serde_json::json!({});
        let verdict: Verdict = serde_json::from_value(json).unwrap();
        assert_eq!(
            check(&verdict, PACKAGE, "h", NOW),
            Err(IntegrityError::Refused("device"))
        );
    }

    #[test]
    fn old_or_future_verdicts_are_refused() {
        assert_eq!(
            check(&verdict("h", NOW - 11 * 60_000), PACKAGE, "h", NOW),
            Err(IntegrityError::Refused("stale"))
        );
        assert_eq!(
            check(&verdict("h", NOW + 2 * 60_000), PACKAGE, "h", NOW),
            Err(IntegrityError::Refused("stale"))
        );
    }

    #[test]
    fn a_verdict_without_request_details_does_not_parse() {
        assert!(serde_json::from_value::<Verdict>(serde_json::json!({})).is_err());
    }

    async fn google(server: &MockServer) -> GoogleIntegrity {
        let base = Url::parse(&format!("{}/", server.uri())).unwrap();
        GoogleIntegrity::with_endpoints(PACKAGE.into(), base.clone(), base)
    }

    async fn mock_metadata(server: &MockServer, times: u64) {
        Mock::given(method("GET"))
            .and(path(
                "/computeMetadata/v1/instance/service-accounts/default/token",
            ))
            .and(query_param(
                "scopes",
                "https://www.googleapis.com/auth/playintegrity",
            ))
            .and(header("metadata-flavor", "Google"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "ya29.test", "expires_in": 3599, "token_type": "Bearer"
            })))
            .expect(times)
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn google_decodes_the_token_with_the_service_account() {
        let server = MockServer::start().await;
        mock_metadata(&server, 1).await;
        let now = now_millis();
        Mock::given(method("POST"))
            .and(path(format!("/v1/{PACKAGE}:decodeIntegrityToken")))
            .and(header("authorization", "Bearer ya29.test"))
            .and(body_json(serde_json::json!({"integrityToken": "tok"})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"tokenPayloadExternal": verdict_json("h", now)}),
                ),
            )
            .expect(2)
            .mount(&server)
            .await;
        let integrity = Integrity::Google(google(&server).await);
        assert_eq!(integrity.verify(Some("tok"), "h").await, Ok(()));
        // The access token is cached: the metadata server is asked once.
        assert_eq!(integrity.verify(Some("tok"), "h").await, Ok(()));
    }

    #[tokio::test]
    async fn a_missing_token_is_refused_without_asking_google() {
        let server = MockServer::start().await;
        mock_metadata(&server, 0).await;
        let integrity = Integrity::Google(google(&server).await);
        assert_eq!(
            integrity.verify(None, "h").await,
            Err(IntegrityError::Missing)
        );
        assert_eq!(
            integrity.verify(Some(""), "h").await,
            Err(IntegrityError::Missing)
        );
        let huge = "x".repeat(MAX_TOKEN_BYTES + 1);
        assert_eq!(
            integrity.verify(Some(&huge), "h").await,
            Err(IntegrityError::Missing)
        );
    }

    #[tokio::test]
    async fn an_invalid_token_is_refused() {
        let server = MockServer::start().await;
        mock_metadata(&server, 1).await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&server)
            .await;
        let integrity = Integrity::Google(google(&server).await);
        assert_eq!(
            integrity.verify(Some("bad"), "h").await,
            Err(IntegrityError::Refused("token"))
        );
    }

    #[tokio::test]
    async fn google_being_down_is_not_the_apps_fault() {
        let server = MockServer::start().await;
        mock_metadata(&server, 1).await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let integrity = Integrity::Google(google(&server).await);
        assert_eq!(
            integrity.verify(Some("tok"), "h").await,
            Err(IntegrityError::Unavailable)
        );
    }

    #[tokio::test]
    async fn rejected_credentials_are_not_the_apps_fault_and_are_not_cached() {
        let server = MockServer::start().await;
        mock_metadata(&server, 2).await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;
        let integrity = Integrity::Google(google(&server).await);
        for _ in 0..2 {
            assert_eq!(
                integrity.verify(Some("tok"), "h").await,
                Err(IntegrityError::Unavailable)
            );
        }
    }

    #[tokio::test]
    async fn an_exhausted_quota_is_unavailable() {
        let server = MockServer::start().await;
        mock_metadata(&server, 1).await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;
        let integrity = Integrity::Google(google(&server).await);
        assert_eq!(
            integrity.verify(Some("tok"), "h").await,
            Err(IntegrityError::Unavailable)
        );
    }

    #[tokio::test]
    async fn an_unparseable_answer_is_unavailable() {
        let server = MockServer::start().await;
        mock_metadata(&server, 1).await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;
        let integrity = Integrity::Google(google(&server).await);
        assert_eq!(
            integrity.verify(Some("tok"), "h").await,
            Err(IntegrityError::Unavailable)
        );
    }

    #[tokio::test]
    async fn off_accepts_everything() {
        assert_eq!(Integrity::Off.verify(None, "h").await, Ok(()));
    }
}
