//! Expanding Google Maps short links (spec §4, decision 31). Each redirect is followed by hand;
//! only links the core recognises as short links are ever fetched. Nothing is cached or logged.

use std::time::Duration;

use axum::Json;
use axum::extract::State;
use carsharing_core::{Destination, parse_destination};
use reqwest::header::LOCATION;
use serde::Deserialize;
use url::Url;

use crate::auth::Authed;
use crate::error::ApiError;
use crate::limits::{self, Kind};
use crate::state::AppState;

pub const MAX_HOPS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandError {
    NotAShortLink,
    NoDestination,
    Unavailable,
}

impl From<ExpandError> for ApiError {
    fn from(error: ExpandError) -> ApiError {
        match error {
            ExpandError::NotAShortLink => ApiError::NotAShortLink,
            ExpandError::NoDestination => ApiError::NoDestination,
            ExpandError::Unavailable => ApiError::RouteServiceBusy,
        }
    }
}

pub struct Expander {
    http: reqwest::Client,
    origin: Option<Url>,
}

impl Expander {
    pub fn new() -> Expander {
        Expander {
            http: crate::http::client_without_redirects(Duration::from_secs(5)),
            origin: None,
        }
    }

    /// Sends every request to `origin` instead, keeping the path; for tests.
    pub fn with_origin(origin: Url) -> Expander {
        Expander {
            origin: Some(origin),
            ..Expander::new()
        }
    }

    pub async fn expand(&self, link: &str) -> Result<Destination, ExpandError> {
        let Ok(Destination::ShortLink { url }) = parse_destination(link) else {
            return Err(ExpandError::NotAShortLink);
        };
        let mut next = url;
        for _ in 0..MAX_HOPS {
            let location = self.follow(&next).await?;
            let target = unwrap_consent(&location).unwrap_or(location);
            match parse_destination(&target) {
                Ok(Destination::ShortLink { url }) => next = url,
                Ok(destination) => return Ok(destination),
                Err(_) => return Err(ExpandError::NoDestination),
            }
        }
        tracing::warn!("short link redirects too often");
        Err(ExpandError::Unavailable)
    }

    /// The redirect target of one short link, resolved against it.
    async fn follow(&self, short: &str) -> Result<String, ExpandError> {
        let short = Url::parse(short).map_err(|_| ExpandError::NotAShortLink)?;
        let fetch = match &self.origin {
            Some(origin) => origin
                .join(short.path())
                .map_err(|_| ExpandError::Unavailable)?,
            None => short.clone(),
        };
        let response = self
            .http
            .get(fetch)
            .header("User-Agent", "whichride-server")
            .send()
            .await
            .map_err(|_| {
                tracing::warn!("short link host unreachable or timed out");
                ExpandError::Unavailable
            })?;
        let status = response.status();
        if status.is_server_error() {
            tracing::warn!(status = status.as_u16(), "short link host failed");
            return Err(ExpandError::Unavailable);
        }
        if !status.is_redirection() {
            return Err(ExpandError::NoDestination);
        }
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(ExpandError::NoDestination)?;
        let resolved = short
            .join(location)
            .map_err(|_| ExpandError::NoDestination)?;
        Ok(resolved.into())
    }
}

impl Default for Expander {
    fn default() -> Expander {
        Expander::new()
    }
}

/// From Europe, Google may redirect to its cookie consent page first; the real target is its
/// `continue` parameter (Review Focus 2).
fn unwrap_consent(location: &str) -> Option<String> {
    let url = Url::parse(location).ok()?;
    let host = url.host_str()?;
    if !host.starts_with("consent.google.") {
        return None;
    }
    url.query_pairs()
        .find(|(key, _)| key == "continue")
        .map(|(_, value)| value.into_owned())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpandRequest {
    pub url: String,
}

pub async fn post_expand(
    State(state): State<AppState>,
    Authed { install, body }: Authed<ExpandRequest>,
) -> Result<Json<Destination>, ApiError> {
    limits::check(&state, &install, Kind::Expand)?;
    Ok(Json(state.expander.expand(&body.url).await?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_consent_page_leads_to_its_continue_target() {
        let location = "https://consent.google.com/ml?continue=https%3A%2F%2Fwww.google.com%2Fmaps%2Fplace%2FX%2F%4052.52%2C13.41%2C15z&gl=DE&hl=de&pc=m&src=1";
        assert_eq!(
            unwrap_consent(location).as_deref(),
            Some("https://www.google.com/maps/place/X/@52.52,13.41,15z")
        );
        assert_eq!(
            unwrap_consent("https://consent.google.de/ml?continue=https://www.google.de/maps")
                .as_deref(),
            Some("https://www.google.de/maps")
        );
    }

    #[test]
    fn other_pages_are_not_consent_pages() {
        assert_eq!(
            unwrap_consent("https://www.google.com/maps?continue=x"),
            None
        );
        assert_eq!(unwrap_consent("https://consent.google.com/ml"), None);
        assert_eq!(
            unwrap_consent("https://consent.evil.com/ml?continue=x"),
            None
        );
        assert_eq!(unwrap_consent("not a url"), None);
    }
}
