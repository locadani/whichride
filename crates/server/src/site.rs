//! The website (milestone 7): the static files Astro built, served after the API routes.
//! Only the route template ("unmatched") is logged, as for the API.

use std::path::Path;

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_status::SetStatus;

use crate::error::ApiError;

/// The site's languages; the first is the default.
pub const LANGUAGES: [&str; 4] = ["en", "it", "de", "tr"];

/// Files a built site must have; `serve` refuses a `WEB_DIR` without them.
const REQUIRED: [&str; 2] = ["en/index.html", "404.html"];

pub fn check(dir: &Path) -> Result<(), String> {
    for file in REQUIRED {
        if !dir.join(file).is_file() {
            return Err(format!(
                "{} has no {file}: build the site (web/: npm run build)",
                dir.display()
            ));
        }
    }
    Ok(())
}

/// `/` → the visitor's language, unknown `/v1/…` → the API's JSON 404, everything else from `dir`.
pub fn router(dir: &Path) -> Router {
    let not_found = SetStatus::new(ServeFile::new(dir.join("404.html")), StatusCode::NOT_FOUND);
    let files = ServeDir::new(dir)
        .precompressed_br()
        .precompressed_gzip()
        .not_found_service(not_found);
    Router::new()
        .route("/", get(to_language))
        .route("/v1/{*rest}", any(|| async { ApiError::NotFound }))
        .fallback_service(files)
        .layer(middleware::from_fn(site_headers))
}

async fn to_language(headers: HeaderMap) -> Response {
    let accept = headers
        .get(header::ACCEPT_LANGUAGE)
        .and_then(|v| v.to_str().ok());
    (
        StatusCode::FOUND,
        [
            (header::LOCATION, format!("/{}/", best_language(accept))),
            (header::VARY, "Accept-Language".to_owned()),
        ],
    )
        .into_response()
}

/// The best of `LANGUAGES` for an `Accept-Language` header: highest q wins, ties go to the earlier
/// entry, q=0 and broken entries are skipped; `en` when nothing matches.
pub fn best_language(header: Option<&str>) -> &'static str {
    let mut best: Option<(&'static str, u16)> = None;
    for item in header.unwrap_or_default().split(',') {
        let mut parts = item.split(';');
        let primary = parts
            .next()
            .unwrap_or_default()
            .trim()
            .split('-')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let quality = match parts.find_map(|p| p.trim().strip_prefix("q=")) {
            None => Some(1000),
            Some(q) => parse_quality(q),
        };
        let (Some(quality), Some(language)) = (quality, LANGUAGES.iter().find(|l| **l == primary))
        else {
            continue;
        };
        if quality > 0 && best.is_none_or(|(_, b)| quality > b) {
            best = Some((language, quality));
        }
    }
    best.map_or(LANGUAGES[0], |(language, _)| language)
}

/// "0.8" → 800; outside 0..=1 or not a number → None.
fn parse_quality(text: &str) -> Option<u16> {
    let value: f32 = text.trim().parse().ok()?;
    (0.0..=1.0)
        .contains(&value)
        .then(|| (value * 1000.0).round() as u16)
}

/// Hashed assets are cached for a year; everything else revalidates. Hardening headers for all.
async fn site_headers(request: Request, next: Next) -> Response {
    let hashed = request.uri().path().starts_with("/_astro/");
    let mut response = next.run(request).await;
    let immutable = hashed && response.status().is_success();
    let headers = response.headers_mut();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if immutable {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        }),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    response
}

#[cfg(test)]
mod tests {
    use super::best_language;

    #[test]
    fn the_best_supported_language_wins() {
        assert_eq!(best_language(Some("de-DE,de;q=0.9")), "de");
        assert_eq!(best_language(Some("en;q=0.2, it;q=0.8")), "it");
        assert_eq!(best_language(Some("TR")), "tr");
        assert_eq!(
            best_language(Some("it;q=0, de;q=0.1")),
            "de",
            "q=0 means not acceptable"
        );
        assert_eq!(
            best_language(Some("de;q=0.5, it;q=0.5")),
            "de",
            "ties go to the earlier entry"
        );
        assert_eq!(best_language(Some("*")), "en");
        assert_eq!(
            best_language(Some("it;q=abc")),
            "en",
            "a broken q is ignored"
        );
        assert_eq!(best_language(Some("")), "en");
        assert_eq!(best_language(None), "en");
    }
}
