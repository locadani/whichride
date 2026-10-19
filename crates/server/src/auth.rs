//! Who is calling: install tokens (decision 32) and the extractors that check them together
//! with Play Integrity (decision 33).

use axum::body::{Bytes, to_bytes};
use axum::extract::{FromRequest, Request};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use carsharing_core::{Day, InstallId, InstallRecord};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::app::BODY_LIMIT_BYTES;
use crate::error::ApiError;
use crate::integrity::{TOKEN_HEADER, request_hash};
use crate::limits::{self, Kind};
use crate::state::AppState;

const TOKEN_BYTES: usize = 32;

/// A new bearer token: 32 random bytes, base64url without padding (43 characters).
pub fn new_token() -> Result<String, ApiError> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| ApiError::internal(format!("random: {e}")))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn hash_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// A registered install and its voting history.
#[derive(Debug, Clone)]
pub struct Install {
    pub id: Uuid,
    pub record: InstallRecord,
}

impl Install {
    pub fn install_id(&self) -> InstallId {
        InstallId(self.id.to_string())
    }
}

pub async fn find_install(pool: &PgPool, token: &str) -> sqlx::Result<Option<Install>> {
    let row: Option<(Uuid, i32, i32, i32)> = sqlx::query_as(
        "SELECT id, first_seen, decided_votes, matching_votes FROM installs WHERE token_hash = $1",
    )
    .bind(hash_token(token).as_slice())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id, first_seen, decided, matching)| Install {
        id,
        record: InstallRecord {
            first_seen: Day(first_seen),
            decided_votes: u32::try_from(decided).unwrap_or(0),
            matching_votes: u32::try_from(matching).unwrap_or(0),
        },
    }))
}

/// A JSON body with a valid integrity token for this exact request. Registration's extractor:
/// it counts against the hourly registration limit before Google is asked.
pub struct Attested<T>(pub T);

/// A JSON body from a registered install, with a valid integrity token for this exact request
/// of this install. Counts against the install's hourly write limit before Google is asked.
pub struct Signed<T> {
    pub install: Install,
    pub body: T,
}

/// A JSON body from a registered install.
pub struct Authed<T> {
    pub install: Install,
    pub body: T,
}

impl<T: DeserializeOwned + Send> FromRequest<AppState> for Attested<T> {
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &AppState) -> Result<Self, ApiError> {
        let (parts, bytes) = split(request).await?;
        let body = parse(&bytes)?;
        limits::check_registration(state)?;
        attest(&parts, &bytes, None, state).await?;
        Ok(Attested(body))
    }
}

impl<T: DeserializeOwned + Send> FromRequest<AppState> for Signed<T> {
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &AppState) -> Result<Self, ApiError> {
        let (parts, bytes) = split(request).await?;
        let install = authenticate(&parts, state).await?;
        let body = parse(&bytes)?;
        limits::check(state, &install, Kind::Write)?;
        attest(&parts, &bytes, Some(install.id), state).await?;
        Ok(Signed { install, body })
    }
}

impl<T: DeserializeOwned + Send> FromRequest<AppState> for Authed<T> {
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &AppState) -> Result<Self, ApiError> {
        let (parts, bytes) = split(request).await?;
        let install = authenticate(&parts, state).await?;
        let body = parse(&bytes)?;
        Ok(Authed { install, body })
    }
}

async fn split(request: Request) -> Result<(Parts, Bytes), ApiError> {
    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, BODY_LIMIT_BYTES)
        .await
        .map_err(|_| ApiError::BadRequest)?;
    Ok((parts, bytes))
}

fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ApiError> {
    serde_json::from_slice(bytes).map_err(|_| ApiError::BadRequest)
}

async fn authenticate(parts: &Parts, state: &AppState) -> Result<Install, ApiError> {
    let token = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| token.len() <= 64)
        .ok_or(ApiError::Unauthorized)?;
    find_install(&state.db, token)
        .await?
        .ok_or(ApiError::Unauthorized)
}

/// Checks the integrity token against this request; a registered install's ID is part of it.
async fn attest(
    parts: &Parts,
    body: &[u8],
    install: Option<Uuid>,
    state: &AppState,
) -> Result<(), ApiError> {
    let token = parts
        .headers
        .get(TOKEN_HEADER)
        .and_then(|value| value.to_str().ok());
    let hash = request_hash(parts.method.as_str(), parts.uri.path(), install, body);
    state.integrity.verify(token, &hash).await?;
    Ok(())
}
