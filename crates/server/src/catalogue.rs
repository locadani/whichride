//! `GET /v1/catalogue`: what the app and the website compare with.

use std::collections::HashMap;

use axum::Json;
use axum::extract::State;
use carsharing_core::{Catalogue, Day, Offer, OfferId, Provider, ProviderId, RateCard};
use serde::Serialize;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use crate::db;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct CatalogueResponse {
    pub catalogue: Catalogue,
    pub offers: Vec<OfferStatus>,
    pub proposals: Vec<Proposal>,
}

#[derive(Debug, Serialize)]
pub struct OfferStatus {
    pub offer: OfferId,
    pub confirmed_on: Option<Day>,
    pub locked: bool,
    pub pending: Option<PendingReport>,
}

#[derive(Debug, Serialize)]
pub struct PendingReport {
    pub report: Uuid,
    pub cards: Vec<RateCard>,
}

#[derive(Debug, Serialize)]
pub struct Proposal {
    pub report: Uuid,
    pub provider: ProviderId,
    pub name: String,
    pub cards: Vec<RateCard>,
}

pub async fn get_catalogue(
    State(state): State<AppState>,
) -> Result<Json<CatalogueResponse>, ApiError> {
    let offers = db::list_offers(&state.db).await?;
    let mut pending = oldest_pending_reports(&state).await?;
    let proposals = pending_proposals(&state).await?;

    let mut providers = Vec::new();
    let mut statuses = Vec::new();
    for seed in &state.providers.providers {
        let mut provider = Provider {
            offers: Vec::new(),
            ..seed.clone()
        };
        for row in offers.iter().filter(|row| row.provider_id == seed.id.0) {
            provider.offers.push(Offer {
                id: OfferId(row.id.clone()),
                name: row.name.clone(),
                cards: row.cards.0.clone(),
            });
            statuses.push(OfferStatus {
                offer: OfferId(row.id.clone()),
                confirmed_on: row.confirmed_on.map(Day),
                locked: row.locked,
                pending: pending.remove(&row.id),
            });
        }
        if !provider.offers.is_empty() {
            providers.push(provider);
        }
    }
    Ok(Json(CatalogueResponse {
        catalogue: Catalogue { providers },
        offers: statuses,
        proposals,
    }))
}

/// The oldest pending report of each offer.
async fn oldest_pending_reports(
    state: &AppState,
) -> Result<HashMap<String, PendingReport>, ApiError> {
    let rows: Vec<(String, Uuid, SqlJson<Vec<RateCard>>)> = sqlx::query_as(
        "SELECT DISTINCT ON (offer_id) offer_id, id, cards FROM reports
         WHERE status = 'pending' AND offer_id IS NOT NULL
         ORDER BY offer_id, created_on, id",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(offer, report, cards)| {
            (
                offer,
                PendingReport {
                    report,
                    cards: cards.0,
                },
            )
        })
        .collect())
}

async fn pending_proposals(state: &AppState) -> Result<Vec<Proposal>, ApiError> {
    let rows: Vec<(Uuid, String, String, SqlJson<Vec<RateCard>>)> = sqlx::query_as(
        "SELECT id, proposed_provider, proposed_name, cards FROM reports
         WHERE status = 'pending' AND offer_id IS NULL
         ORDER BY created_on, id",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|(_, provider, _, _)| {
            state
                .providers
                .provider(&ProviderId(provider.clone()))
                .is_some()
        })
        .map(|(report, provider, name, cards)| Proposal {
            report,
            provider: ProviderId(provider),
            name,
            cards: cards.0,
        })
        .collect())
}
