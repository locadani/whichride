//! Play Integrity (spec §7.4): a write is accepted only from a genuine app on a genuine device,
//! and only for the exact request the app asked Google to vouch for.

use std::collections::{HashSet, VecDeque};
use std::sync::Mutex as SyncMutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

use crate::error::ApiError;

pub const TOKEN_HEADER: &str = "x-integrity-token";

/// Real tokens are a few kilobytes; anything longer is not sent to Google.
pub const MAX_TOKEN_BYTES: usize = 8 * 1024;

/// A verdict older than this is refused; the app asks for a token right before each write.
const MAX_AGE_MILLIS: i64 = 10 * 60 * 1000;
/// Tolerated difference between Google's clock and ours.
const MAX_FUTURE_MILLIS: i64 = 60 * 1000;

const SCOPE: &str = "https://www.googleapis.com/auth/playintegrity";

/// How long an accepted token is remembered: longer than any verdict for it stays fresh.
const SEEN_FOR: Duration = Duration::from_millis((MAX_AGE_MILLIS + MAX_FUTURE_MILLIS) as u64);
/// The most tokens remembered at once; when full, new verifications are refused rather than
/// letting the memory grow.
const MAX_SEEN: usize = 100_000;

/// Lowercase hex SHA-256 of `"{method} {path}\n"`, then — for a registered install's request —
/// its ID (lowercase, hyphenated) and `"\n"`, then the body. The app sets it as the token's
/// `requestHash`, so a token is good for this one request of this one install (decision 44).
pub fn request_hash(method: &str, path: &str, install: Option<Uuid>, body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(method.as_bytes());
    hasher.update(b" ");
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    if let Some(install) = install {
        hasher.update(install.hyphenated().to_string().as_bytes());
        hasher.update(b"\n");
    }
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
        // Not worth a call to Google, whose daily allowance is small.
        if !looks_like_a_token(token) {
            return Err(IntegrityError::Refused("format"));
        }
        let seen = Sha256::digest(token.as_bytes()).into();
        google.seen().check(&seen, Instant::now())?;
        let verdict = google.decode(token).await?;
        check(&verdict, &google.package, expected_hash, now_millis())?;
        google.seen().insert(seen, Instant::now())
    }
}

/// Play Integrity tokens are JWE compact: five non-empty base64url parts separated by dots.
/// Each part must be spelled the one canonical way (no padding, unused bits zero), so a used
/// token can't come back respelled with a new hash and slip past [`SeenTokens`].
fn looks_like_a_token(token: &str) -> bool {
    let mut parts = 0;
    for part in token.split('.') {
        parts += 1;
        if part.is_empty() || URL_SAFE_NO_PAD.decode(part).is_err() {
            return false;
        }
    }
    parts == 5
}

/// The SHA-256 of every token accepted recently, so each token is accepted once.
struct SeenTokens {
    capacity: usize,
    hashes: HashSet<[u8; 32]>,
    /// In the order accepted, with the moment each may be forgotten.
    expiry: VecDeque<(Instant, [u8; 32])>,
}

impl SeenTokens {
    fn new(capacity: usize) -> SeenTokens {
        SeenTokens {
            capacity,
            hashes: HashSet::new(),
            expiry: VecDeque::new(),
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.hashes.len()
    }

    fn forget_expired(&mut self, now: Instant) {
        while let Some((expires, hash)) = self.expiry.front().copied()
            && expires <= now
        {
            self.expiry.pop_front();
            self.hashes.remove(&hash);
        }
    }

    /// Before asking Google: a token seen before is refused, and nothing is verified while full.
    fn check(&mut self, hash: &[u8; 32], now: Instant) -> Result<(), IntegrityError> {
        self.forget_expired(now);
        if self.hashes.contains(hash) {
            return Err(IntegrityError::Refused("replayed"));
        }
        if self.hashes.len() >= self.capacity {
            tracing::warn!("too many integrity tokens to remember");
            return Err(IntegrityError::Unavailable);
        }
        Ok(())
    }

    /// After a good verdict; checks again, as another request may have used the token since.
    fn insert(&mut self, hash: [u8; 32], now: Instant) -> Result<(), IntegrityError> {
        self.check(&hash, now)?;
        self.hashes.insert(hash);
        self.expiry.push_back((now + SEEN_FOR, hash));
        Ok(())
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
    seen: SyncMutex<SeenTokens>,
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
            seen: SyncMutex::new(SeenTokens::new(MAX_SEEN)),
        }
    }

    fn seen(&self) -> std::sync::MutexGuard<'_, SeenTokens> {
        self.seen
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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
    /// Shaped like a real token (JWE compact).
    const TOKEN: &str = "eyJhbGciOiJBMjU2S1cifQ.a2V5.aXY.Y2lwaGVydGV4dA.dGFn";
    const OTHER_TOKEN: &str = "eyJhbGciOiJBMjU2S1cifQ.a2V5Mg.aXY.Y2lwaGVydGV4dA.dGFn";
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

    const INSTALL: &str = "123e4567-e89b-42d3-a456-426614174000";

    #[test]
    fn the_request_hash_covers_method_path_install_and_body() {
        let install = Uuid::parse_str(INSTALL).unwrap();
        let hash = request_hash("POST", "/v1/reports", Some(install), br#"{"a":1}"#);
        assert_eq!(hash.len(), 64);
        assert_eq!(
            hash,
            request_hash("POST", "/v1/reports", Some(install), br#"{"a":1}"#)
        );
        assert_ne!(
            hash,
            request_hash("POST", "/v1/reports", Some(install), br#"{"a":2}"#)
        );
        assert_ne!(
            hash,
            request_hash("POST", "/v1/installs", Some(install), br#"{"a":1}"#)
        );
        assert_ne!(
            hash,
            request_hash("POST", "/v1/reports", Some(Uuid::new_v4()), br#"{"a":1}"#)
        );
        assert_ne!(
            hash,
            request_hash("POST", "/v1/reports", None, br#"{"a":1}"#)
        );
    }

    /// Known values computed independently, so the Android implementation can be checked
    /// against them:
    /// `printf 'POST /v1/installs\n{"nonce":"abcdefghijklmnop"}' | shasum -a 256` and
    /// `printf 'POST /v1/offers/miles.s/confirmations\n123e4567-e89b-42d3-a456-426614174000\n{"answer":"yes"}' | shasum -a 256`.
    #[test]
    fn the_request_hash_has_known_values() {
        assert_eq!(
            request_hash(
                "POST",
                "/v1/installs",
                None,
                br#"{"nonce":"abcdefghijklmnop"}"#
            ),
            "c59b7c5ccbba73f1c1f2c526838ae9486eaaf2ac726b320a170aad41b31565c3"
        );
        assert_eq!(
            request_hash(
                "POST",
                "/v1/offers/miles.s/confirmations",
                Some(Uuid::parse_str(INSTALL).unwrap()),
                br#"{"answer":"yes"}"#
            ),
            "78a4418304e69be8eebd247ac4e93fc97b3fca818a814017cfef1d82be803413"
        );
    }

    #[test]
    fn a_token_is_seen_until_it_expires() {
        let start = Instant::now();
        let mut seen = SeenTokens::new(10);
        let token = [1u8; 32];
        assert_eq!(seen.check(&token, start), Ok(()));
        assert_eq!(seen.insert(token, start), Ok(()));
        assert_eq!(
            seen.check(&token, start + Duration::from_secs(60)),
            Err(IntegrityError::Refused("replayed"))
        );
        assert_eq!(
            seen.insert(token, start + Duration::from_secs(60)),
            Err(IntegrityError::Refused("replayed"))
        );
        // Long after any verdict for it could still be fresh, it is forgotten.
        let later = start + SEEN_FOR + Duration::from_secs(1);
        assert_eq!(seen.check(&token, later), Ok(()));
        assert_eq!(seen.len(), 0);
    }

    #[test]
    fn a_full_cache_refuses_new_tokens_instead_of_growing() {
        let start = Instant::now();
        let mut seen = SeenTokens::new(2);
        assert_eq!(seen.insert([1; 32], start), Ok(()));
        assert_eq!(seen.insert([2; 32], start), Ok(()));
        assert_eq!(
            seen.check(&[3; 32], start),
            Err(IntegrityError::Unavailable)
        );
        assert_eq!(
            seen.insert([3; 32], start),
            Err(IntegrityError::Unavailable)
        );
        assert_eq!(seen.len(), 2);
        // Expired entries make room again.
        let later = start + SEEN_FOR + Duration::from_secs(1);
        assert_eq!(seen.insert([3; 32], later), Ok(()));
        assert_eq!(seen.len(), 1);
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
        for token in [TOKEN, OTHER_TOKEN] {
            Mock::given(method("POST"))
                .and(path(format!("/v1/{PACKAGE}:decodeIntegrityToken")))
                .and(header("authorization", "Bearer ya29.test"))
                .and(body_json(serde_json::json!({ "integrityToken": token })))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"tokenPayloadExternal": verdict_json("h", now)}),
                ))
                .expect(1)
                .mount(&server)
                .await;
        }
        let integrity = Integrity::Google(google(&server).await);
        assert_eq!(integrity.verify(Some(TOKEN), "h").await, Ok(()));
        // The access token is cached: the metadata server is asked once.
        assert_eq!(integrity.verify(Some(OTHER_TOKEN), "h").await, Ok(()));
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
    async fn a_token_not_shaped_like_one_is_refused_without_asking_google() {
        let server = MockServer::start().await;
        mock_metadata(&server, 0).await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&server)
            .await;
        let integrity = Integrity::Google(google(&server).await);
        for token in [
            "tok",
            "a.b.c.d",
            "a.b.c.d.e.f",
            "a..c.d.e",
            ".b.c.d.e",
            "a.b.c.d.",
            "a.b.c.d.e+",
            "a.b.c.d.e=",
            "a.b/c.d.e",
            "a.b.c.d.é",
            "eyJhbGciOiJBMjU2S1cifQ.a2V5.aXZ.Y2lwaGVydGV4dA.dGFn",
        ] {
            assert_eq!(
                integrity.verify(Some(token), "h").await,
                Err(IntegrityError::Refused("format")),
                "{token}"
            );
        }
    }

    #[test]
    fn real_tokens_are_jwe_compact() {
        assert!(looks_like_a_token(TOKEN));
        assert!(looks_like_a_token("A-z_09AA.bA.cA.dA.eA"));
        assert!(!looks_like_a_token("aA.bA.cA.dA"));
    }

    #[test]
    fn each_token_has_one_spelling() {
        // "aXY" and "aXZ" differ only in bits base64url drops, so a lenient decoder reads both
        // as "iv": a used token respelled this way would pass the once-only check.
        assert!(looks_like_a_token(TOKEN));
        assert!(!looks_like_a_token(&TOKEN.replace(".aXY.", ".aXZ.")));
        // One character can't hold a byte.
        assert!(!looks_like_a_token(&TOKEN.replace(".aXY.", ".a.")));
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
            integrity.verify(Some(TOKEN), "h").await,
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
            integrity.verify(Some(TOKEN), "h").await,
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
                integrity.verify(Some(TOKEN), "h").await,
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
            integrity.verify(Some(TOKEN), "h").await,
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
            integrity.verify(Some(TOKEN), "h").await,
            Err(IntegrityError::Unavailable)
        );
    }

    #[tokio::test]
    async fn off_accepts_everything() {
        assert_eq!(Integrity::Off.verify(None, "h").await, Ok(()));
    }
}
