//! `POST /v1/installs`: a genuine app gets its anonymous identity (spec §7.4).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{Attested, hash_token, new_token};
use crate::error::ApiError;
use crate::state::AppState;

/// Registration carries nothing; the body exists so the integrity token can be bound to it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterRequest {}

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
