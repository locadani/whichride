//! OpenRouteService behind our key (spec §8). Floats exist only here, at the boundary: they are
//! converted to whole metres, seconds and 1e-7 degrees right away. Request data never appears
//! in an error or a log line: errors are mapped to three kinds and only the status is logged.

use std::time::Duration;

use carsharing_core::{Coordinate, Trip};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::ApiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    Geocode,
    Autocomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Place {
    pub label: String,
    pub at: Coordinate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrsError {
    /// Quota used up or rate limited (403, 429).
    Busy,
    /// No route between the points (404, 400).
    NoRoute,
    /// Down, slow or answering nonsense.
    Unavailable,
}

impl From<OrsError> for ApiError {
    fn from(error: OrsError) -> ApiError {
        match error {
            OrsError::NoRoute => ApiError::NoRoute,
            OrsError::Busy | OrsError::Unavailable => ApiError::RouteServiceBusy,
        }
    }
}

/// Places asked for per search.
const RESULTS: &str = "5";

pub struct Ors {
    http: reqwest::Client,
    base: Url,
    key: String,
}

#[derive(Deserialize)]
struct Directions {
    routes: Vec<Route>,
}

#[derive(Deserialize)]
struct Route {
    summary: Summary,
}

#[derive(Deserialize)]
struct Summary {
    #[serde(default)]
    distance: f64,
    #[serde(default)]
    duration: f64,
}

#[derive(Deserialize)]
struct Features {
    features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
    properties: Properties,
    geometry: Geometry,
}

#[derive(Deserialize)]
struct Properties {
    #[serde(default)]
    label: Option<String>,
}

#[derive(Deserialize)]
struct Geometry {
    coordinates: (f64, f64),
}

impl Ors {
    pub fn new(base: Url, key: String) -> Ors {
        Ors {
            http: crate::http::client(Duration::from_secs(8)),
            base,
            key,
        }
    }

    pub fn with_timeout(self, timeout: Duration) -> Ors {
        Ors {
            http: crate::http::client(timeout),
            ..self
        }
    }

    pub async fn route(&self, from: Coordinate, to: Coordinate) -> Result<Trip, OrsError> {
        let url = self
            .base
            .join("v2/directions/driving-car")
            .map_err(|_| OrsError::Unavailable)?;
        let body = serde_json::json!({
            "coordinates": [
                [degrees(from.lon_e7), degrees(from.lat_e7)],
                [degrees(to.lon_e7), degrees(to.lat_e7)],
            ]
        });
        let response = self
            .http
            .post(url)
            .header("Authorization", &self.key)
            .json(&body)
            .send()
            .await;
        let directions: Directions = read(response, true).await?;
        let summary = &directions.routes.first().ok_or(OrsError::NoRoute)?.summary;
        Ok(Trip {
            metres: whole(summary.distance).ok_or(OrsError::Unavailable)?,
            seconds: whole(summary.duration).ok_or(OrsError::Unavailable)?,
        })
    }

    pub async fn search(
        &self,
        kind: SearchKind,
        text: &str,
        near: Option<Coordinate>,
    ) -> Result<Vec<Place>, OrsError> {
        let path = match kind {
            SearchKind::Geocode => "geocode/search",
            SearchKind::Autocomplete => "geocode/autocomplete",
        };
        let url = self.base.join(path).map_err(|_| OrsError::Unavailable)?;
        let mut query = vec![
            ("text", text.to_owned()),
            ("size", RESULTS.to_owned()),
            ("boundary.country", "DE".to_owned()),
        ];
        if let Some(near) = near {
            query.push(("focus.point.lat", degrees(near.lat_e7).to_string()));
            query.push(("focus.point.lon", degrees(near.lon_e7).to_string()));
        }
        let response = self
            .http
            .get(url)
            .header("Authorization", &self.key)
            .query(&query)
            .send()
            .await;
        let features: Features = read(response, false).await?;
        Ok(features
            .features
            .into_iter()
            .filter_map(|feature| {
                let (lon, lat) = feature.geometry.coordinates;
                let at = Coordinate::new(to_e7(lat)?, to_e7(lon)?).ok()?;
                Some(Place {
                    label: feature.properties.label?,
                    at,
                })
            })
            .collect())
    }
}

/// Maps the response to our three errors, logging only the status (never the URL or the body,
/// which can contain the coordinates or the search text).
async fn read<T: for<'de> Deserialize<'de>>(
    response: reqwest::Result<reqwest::Response>,
    routing: bool,
) -> Result<T, OrsError> {
    let Ok(response) = response else {
        tracing::warn!("routing service unreachable or timed out");
        return Err(OrsError::Unavailable);
    };
    let status = response.status().as_u16();
    let error = match status {
        200..=299 => {
            return response.json().await.map_err(|_| {
                tracing::warn!("routing service answered with an unreadable body");
                OrsError::Unavailable
            });
        }
        403 | 429 => OrsError::Busy,
        400 | 404 if routing => OrsError::NoRoute,
        _ => OrsError::Unavailable,
    };
    tracing::warn!(status, ?error, "routing service refused");
    Err(error)
}

fn degrees(e7: i32) -> f64 {
    f64::from(e7) / 1e7
}

fn to_e7(degrees: f64) -> Option<i32> {
    let value = (degrees * 1e7).round();
    (value.is_finite() && value.abs() <= f64::from(i32::MAX)).then_some(value as i32)
}

fn whole(value: f64) -> Option<u32> {
    let value = value.round();
    (value.is_finite() && (0.0..=f64::from(u32::MAX)).contains(&value)).then_some(value as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degrees_convert_both_ways() {
        assert_eq!(degrees(525_219_184), 52.521_918_4);
        assert_eq!(to_e7(52.521_918_4), Some(525_219_184));
        assert_eq!(to_e7(-0.000_000_05), Some(-1)); // rounds half away from zero
        assert_eq!(to_e7(f64::NAN), None);
        assert_eq!(to_e7(1e300), None);
    }

    #[test]
    fn whole_units_round_and_refuse_nonsense() {
        assert_eq!(whole(4_321.5), Some(4_322));
        assert_eq!(whole(0.0), Some(0));
        assert_eq!(whole(-1.0), None);
        assert_eq!(whole(f64::INFINITY), None);
        assert_eq!(whole(1e12), None);
    }
}
