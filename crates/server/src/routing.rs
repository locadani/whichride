//! `/v1/route`, `/v1/geocode`, `/v1/autocomplete`: the app's location data passes through to
//! OpenRouteService and back; nothing is stored, cached or logged (spec §9).

use axum::Json;
use axum::extract::State;
use carsharing_core::{Coordinate, Trip};
use serde::{Deserialize, Serialize};

use crate::auth::Authed;
use crate::error::ApiError;
use crate::limits::{self, Kind};
use crate::ors::{Place, SearchKind};
use crate::state::AppState;

pub const MAX_SEARCH_CHARS: usize = 200;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteRequest {
    pub from: Coordinate,
    pub to: Coordinate,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    pub text: String,
    #[serde(default)]
    pub near: Option<Coordinate>,
}

#[derive(Serialize)]
pub struct Places {
    pub places: Vec<Place>,
}

pub async fn post_route(
    State(state): State<AppState>,
    Authed { install, body }: Authed<RouteRequest>,
) -> Result<Json<Trip>, ApiError> {
    limits::check(&state, &install, Kind::Route)?;
    Ok(Json(state.ors.route(body.from, body.to).await?))
}

pub async fn post_geocode(
    state: State<AppState>,
    request: Authed<SearchRequest>,
) -> Result<Json<Places>, ApiError> {
    search(state, request, SearchKind::Geocode, Kind::Geocode).await
}

pub async fn post_autocomplete(
    state: State<AppState>,
    request: Authed<SearchRequest>,
) -> Result<Json<Places>, ApiError> {
    search(state, request, SearchKind::Autocomplete, Kind::Autocomplete).await
}

async fn search(
    State(state): State<AppState>,
    Authed { install, body }: Authed<SearchRequest>,
    search: SearchKind,
    limit: Kind,
) -> Result<Json<Places>, ApiError> {
    let text = body.text.trim();
    if text.is_empty() || text.chars().count() > MAX_SEARCH_CHARS {
        return Err(ApiError::BadRequest);
    }
    limits::check(&state, &install, limit)?;
    let places = state.ors.search(search, text, body.near).await?;
    Ok(Json(Places { places }))
}
