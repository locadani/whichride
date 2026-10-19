//! `POST /v1/installs`: a genuine app gets its anonymous identity (spec §7.4).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{Attested, hash_token, new_token};
use crate::error::ApiError;
use crate::state::AppState;

/// Registration carries only a random nonce from the app, so that every registration is a
/// different request and its integrity token is bound to it (decision 44).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterRequest {
    pub nonce: Nonce,
}

/// 16 to 128 characters of `[A-Za-z0-9_-]`. Checked while parsing, before Google is asked.
#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
pub struct Nonce(pub String);

impl TryFrom<String> for Nonce {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Nonce, &'static str> {
        let well_formed = (16..=128).contains(&value.len())
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
        if well_formed {
            Ok(Nonce(value))
        } else {
            Err("a nonce is 16 to 128 characters of A-Z, a-z, 0-9, _ and -")
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Registered {
    pub install_id: Uuid,
    pub token: String,
}

pub async fn register(
    State(state): State<AppState>,
    Attested(_): Attested<RegisterRequest>,
) -> Result<(StatusCode, Json<Registered>), ApiError> {
    let id = Uuid::new_v4();
    let token = new_token()?;
    sqlx::query("INSERT INTO installs (id, token_hash, first_seen) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(hash_token(&token).as_slice())
        .bind((state.today)().0)
        .execute(&state.db)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(Registered {
            install_id: id,
            token,
        }),
    ))
}
