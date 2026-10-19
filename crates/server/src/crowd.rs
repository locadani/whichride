//! Crowd prices (spec §7): reports, votes and confirmations. The rules are the core's
//! (`weight`, `is_accepted`, `settle`); this module stores them and serialises concurrent
//! requests by locking the offer row first (see the plan's Task 8). `installs` rows are locked
//! last, in id order, so settlements of different offers cannot deadlock.

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use carsharing_core::catalogue::validate_cards;
use carsharing_core::{
    Answer, Cents, Day, InstallId, OfferId, ProviderId, RateCard, Vote, Weight, is_accepted,
    settle, weight,
};
use serde::{Deserialize, Serialize};
use sqlx::types::Json as SqlJson;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::{Install, Signed};
use crate::error::ApiError;
use crate::state::AppState;

pub const MAX_CARDS: usize = 5;
pub const MAX_LABEL_CHARS: usize = 40;
pub const MAX_NAME_CHARS: usize = 60;
const MAX_ID_SLUG_CHARS: usize = 40;
const MAX_BASE_FEE: Cents = Cents(50_000);
const MAX_PER_UNIT: Cents = Cents(500);
const MAX_MINIMUM: Cents = Cents(5_000);
const MAX_INCLUDED: u32 = 20_000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportRequest {
    #[serde(default)]
    pub offer: Option<OfferId>,
    #[serde(default)]
    pub provider: Option<ProviderId>,
    #[serde(default)]
    pub name: Option<String>,
    pub cards: Vec<RateCard>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Pending,
    Accepted,
    /// The reported values are the current price; it was confirmed instead.
    Current,
}

#[derive(Debug, Serialize)]
pub struct ReportOutcome {
    pub report: Option<Uuid>,
    pub status: ReportStatus,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoteRequest {
    pub answer: Answer,
}

#[derive(Debug, Serialize)]
pub struct VoteOutcome {
    pub status: ReportStatus,
}

/// The rate cards of a report: valid for the core, labelled, and believable.
pub fn check_cards(offer: &OfferId, cards: &[RateCard]) -> Result<(), ApiError> {
    if cards.is_empty() || cards.len() > MAX_CARDS {
        return Err(ApiError::BadRequest);
    }
    validate_cards(offer, cards).map_err(|_| ApiError::BadRequest)?;
    for card in cards {
        let label = card.label.trim();
        let plausible = label == card.label
            && !label.is_empty()
            && label.chars().count() <= MAX_LABEL_CHARS
            && card.base_fee <= MAX_BASE_FEE
            && card.per_km <= MAX_PER_UNIT
            && card.per_minute <= MAX_PER_UNIT
            && card.minimum <= MAX_MINIMUM
            && card.included_km <= MAX_INCLUDED
            && card.included_minutes <= MAX_INCLUDED;
        if !plausible {
            return Err(ApiError::BadRequest);
        }
    }
    Ok(())
}

pub async fn post_report(
    State(state): State<AppState>,
    Signed { install, body }: Signed<ReportRequest>,
) -> Result<(StatusCode, Json<ReportOutcome>), ApiError> {
    let today = (state.today)();
    match (body.offer, body.provider, body.name) {
        (Some(offer), None, None) => {
            check_cards(&offer, &body.cards)?;
            report_on_offer(&state, &install, &offer, &body.cards, today).await
        }
        (None, Some(provider), Some(name)) => {
            let name = name.trim();
            if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
                return Err(ApiError::BadRequest);
            }
            if state.providers.provider(&provider).is_none() {
                return Err(ApiError::NotFound);
            }
            let existing: Option<String> = sqlx::query_scalar(
                "SELECT id FROM offers WHERE provider_id = $1 AND lower(name) = lower($2)",
            )
            .bind(&provider.0)
            .bind(name)
            .fetch_optional(&state.db)
            .await?;
            if let Some(offer) = existing {
                let offer = OfferId(offer);
                check_cards(&offer, &body.cards)?;
                return report_on_offer(&state, &install, &offer, &body.cards, today).await;
            }
            check_cards(&OfferId(offer_id_for(&provider.0, name)), &body.cards)?;
            propose(&state, &install, &provider, name, &body.cards, today).await
        }
        _ => Err(ApiError::BadRequest),
    }
}

async fn report_on_offer(
    state: &AppState,
    install: &Install,
    offer: &OfferId,
    cards: &[RateCard],
    today: Day,
) -> Result<(StatusCode, Json<ReportOutcome>), ApiError> {
    let mut tx = state.db.begin().await?;
    let current = lock_offer(&mut tx, &offer.0).await?;
    let voter_weight = weight(&install.record, today);

    if current.cards.0.as_slice() == cards {
        if voter_weight > Weight(0) {
            confirm(&mut tx, &offer.0, today).await?;
        }
        tx.commit().await?;
        return Ok((
            StatusCode::OK,
            Json(ReportOutcome {
                report: None,
                status: ReportStatus::Current,
            }),
        ));
    }

    let matching: Option<ReportRow> = sqlx::query_as(
        "SELECT id, offer_id, proposed_provider, proposed_name, reporter, reporter_weight,
             cards, status
         FROM reports WHERE offer_id = $1 AND status = 'pending' AND cards = $2
         ORDER BY created_on, id LIMIT 1 FOR UPDATE",
    )
    .bind(&offer.0)
    .bind(SqlJson(cards))
    .fetch_optional(&mut *tx)
    .await?;
    // An install's new report on an offer replaces its own earlier pending one (not the one it
    // joins). Nobody is settled: the reporter changed their own mind.
    sqlx::query(
        "UPDATE reports SET status = 'superseded', decided_on = $3
         WHERE offer_id = $1 AND reporter = $2 AND status = 'pending' AND id IS DISTINCT FROM $4",
    )
    .bind(&offer.0)
    .bind(install.id)
    .bind(today.0)
    .bind(matching.as_ref().map(|report| report.id))
    .execute(&mut *tx)
    .await?;
    if let Some(report) = matching {
        let status = vote_on(&mut tx, &report, install, Answer::Yes, voter_weight, today).await?;
        tx.commit().await?;
        return Ok((
            StatusCode::OK,
            Json(ReportOutcome {
                report: Some(report.id),
                status,
            }),
        ));
    }

    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO reports (id, offer_id, reporter, reporter_weight, cards, created_on)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(&offer.0)
    .bind(install.id)
    .bind(weight_column(voter_weight))
    .bind(SqlJson(cards))
    .bind(today.0)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(ReportOutcome {
            report: Some(id),
            status: ReportStatus::Pending,
        }),
    ))
}

pub async fn post_vote(
    State(state): State<AppState>,
    Path(report_id): Path<String>,
    Signed { install, body }: Signed<VoteRequest>,
) -> Result<Json<VoteOutcome>, ApiError> {
    let report_id = Uuid::parse_str(&report_id).map_err(|_| ApiError::NotFound)?;
    let today = (state.today)();
    let mut tx = state.db.begin().await?;
    // Lock order: the offer first, then the report (see the module docs).
    let target: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT offer_id, proposed_provider FROM reports WHERE id = $1")
            .bind(report_id)
            .fetch_optional(&mut *tx)
            .await?;
    let (offer_id, proposed_provider) = target.ok_or(ApiError::NotFound)?;
    match (&offer_id, &proposed_provider) {
        (Some(offer), _) => {
            lock_offer(&mut tx, offer).await?;
        }
        (None, Some(provider)) => lock_proposals(&mut tx, provider).await?,
        (None, None) => return Err(ApiError::internal("report without a target")),
    }
    let report: ReportRow = sqlx::query_as(
        "SELECT id, offer_id, proposed_provider, proposed_name, reporter, reporter_weight,
             cards, status
         FROM reports WHERE id = $1 FOR UPDATE",
    )
    .bind(report_id)
    .fetch_one(&mut *tx)
    .await?;
    if report.status != "pending" {
        return Err(ApiError::ReportClosed);
    }
    let voter_weight = weight(&install.record, today);
    let status = vote_on(&mut tx, &report, &install, body.answer, voter_weight, today).await?;
    tx.commit().await?;
    Ok(Json(VoteOutcome { status }))
}

pub async fn post_confirmation(
    State(state): State<AppState>,
    Path(offer_id): Path<String>,
    Signed { install, body }: Signed<VoteRequest>,
) -> Result<StatusCode, ApiError> {
    let today = (state.today)();
    let mut tx = state.db.begin().await?;
    lock_offer(&mut tx, &offer_id).await?;
    // Decision 36: a "no" without a value changes nothing.
    if body.answer == Answer::Yes && weight(&install.record, today) > Weight(0) {
        confirm(&mut tx, &offer_id, today).await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct ReportRow {
    pub id: Uuid,
    pub offer_id: Option<String>,
    pub proposed_provider: Option<String>,
    pub proposed_name: Option<String>,
    pub reporter: Uuid,
    pub reporter_weight: i32,
    pub cards: SqlJson<Vec<RateCard>>,
    pub status: String,
}

#[derive(sqlx::FromRow)]
struct LockedOffer {
    cards: SqlJson<Vec<RateCard>>,
    locked: bool,
}

/// Locks an offer row for the rest of the transaction; `NotFound` or `OfferLocked` otherwise.
async fn lock_offer(tx: &mut Transaction<'_, Postgres>, id: &str) -> Result<LockedOffer, ApiError> {
    let offer: LockedOffer =
        sqlx::query_as("SELECT cards, locked FROM offers WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(ApiError::NotFound)?;
    if offer.locked {
        return Err(ApiError::OfferLocked);
    }
    Ok(offer)
}

/// Serialises everything that touches one provider's pending proposals (Task 9).
pub(crate) async fn lock_proposals(
    tx: &mut Transaction<'_, Postgres>,
    provider: &str,
) -> Result<(), ApiError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('proposals:' || $1))")
        .bind(provider)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn confirm(
    tx: &mut Transaction<'_, Postgres>,
    offer: &str,
    today: Day,
) -> Result<(), ApiError> {
    sqlx::query("UPDATE offers SET confirmed_on = $2 WHERE id = $1")
        .bind(offer)
        .bind(today.0)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Records a vote (the reporter's own votes are not stored: they never count) and accepts the
/// report if the core says so.
async fn vote_on(
    tx: &mut Transaction<'_, Postgres>,
    report: &ReportRow,
    voter: &Install,
    answer: Answer,
    voter_weight: Weight,
    today: Day,
) -> Result<ReportStatus, ApiError> {
    if voter.id != report.reporter {
        sqlx::query(
            "INSERT INTO votes (report_id, install_id, answer, weight) VALUES ($1, $2, $3, $4)",
        )
        .bind(report.id)
        .bind(voter.id)
        .bind(answer_column(answer))
        .bind(weight_column(voter_weight))
        .execute(&mut **tx)
        .await?;
    }
    let votes = votes_of(tx, report.id).await?;
    let reporter = InstallId(report.reporter.to_string());
    if is_accepted(
        &reporter,
        weight_from_column(report.reporter_weight),
        &votes,
    ) {
        accept(tx, report, today).await?;
        Ok(ReportStatus::Accepted)
    } else {
        Ok(ReportStatus::Pending)
    }
}

async fn votes_of(tx: &mut Transaction<'_, Postgres>, report: Uuid) -> Result<Vec<Vote>, ApiError> {
    let rows: Vec<(Uuid, String, i32)> = sqlx::query_as(
        "SELECT install_id, answer, weight FROM votes WHERE report_id = $1 ORDER BY seq",
    )
    .bind(report)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(install, answer, weight)| Vote {
            install: InstallId(install.to_string()),
            answer: if answer == "yes" {
                Answer::Yes
            } else {
                Answer::No
            },
            weight: weight_from_column(weight),
        })
        .collect())
}

/// Makes a report the price of its offer — creating the offer for a proposal — supersedes the
/// other pending reports about the same thing, and settles every install involved
/// (decision 39).
pub(crate) async fn accept(
    tx: &mut Transaction<'_, Postgres>,
    report: &ReportRow,
    today: Day,
) -> Result<(), ApiError> {
    sqlx::query("UPDATE reports SET status = 'accepted', decided_on = $2 WHERE id = $1")
        .bind(report.id)
        .bind(today.0)
        .execute(&mut **tx)
        .await?;
    let offer = match &report.offer_id {
        Some(offer) => {
            sqlx::query("UPDATE offers SET cards = $2, confirmed_on = $3 WHERE id = $1")
                .bind(offer)
                .bind(&report.cards)
                .bind(today.0)
                .execute(&mut **tx)
                .await?;
            offer.clone()
        }
        None => create_offer(tx, report, today).await?,
    };
    sqlx::query(
        "INSERT INTO price_history (offer_id, cards, report_id, from_day) VALUES ($1, $2, $3, $4)",
    )
    .bind(&offer)
    .bind(&report.cards)
    .bind(report.id)
    .bind(today.0)
    .execute(&mut **tx)
    .await?;
    let mut deltas = BTreeMap::new();
    settlement(tx, &mut deltas, report.id, report.reporter, true).await?;

    let losers: Vec<(Uuid, Uuid)> = match &report.offer_id {
        Some(offer) => {
            sqlx::query_as(
                "UPDATE reports SET status = 'superseded', decided_on = $2
                 WHERE offer_id = $1 AND status = 'pending' RETURNING id, reporter",
            )
            .bind(offer)
            .bind(today.0)
            .fetch_all(&mut **tx)
            .await?
        }
        None => {
            sqlx::query_as(
                "UPDATE reports SET status = 'superseded', decided_on = $3
                 WHERE offer_id IS NULL AND status = 'pending' AND proposed_provider = $1
                     AND lower(proposed_name) = lower($2)
                 RETURNING id, reporter",
            )
            .bind(&report.proposed_provider)
            .bind(&report.proposed_name)
            .bind(today.0)
            .fetch_all(&mut **tx)
            .await?
        }
    };
    for (loser, reporter) in losers {
        settlement(tx, &mut deltas, loser, reporter, false).await?;
    }
    apply_settlement(tx, &deltas).await
}

/// Adds the settlement of one report (core `settle`) to `deltas`: one decided vote for every
/// install involved, and a matching one for those who were right.
async fn settlement(
    tx: &mut Transaction<'_, Postgres>,
    deltas: &mut BTreeMap<Uuid, (i32, i32)>,
    report: Uuid,
    reporter: Uuid,
    accepted: bool,
) -> Result<(), ApiError> {
    let votes = votes_of(tx, report).await?;
    for (install, was_right) in settle(&InstallId(reporter.to_string()), &votes, accepted) {
        let install = Uuid::parse_str(&install.0).map_err(|e| ApiError::internal(e.to_string()))?;
        let entry = deltas.entry(install).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += i32::from(was_right);
    }
    Ok(())
}

/// Applies settlement deltas in install-id order, so concurrent settlements of different
/// offers lock the same `installs` rows in the same order.
async fn apply_settlement(
    tx: &mut Transaction<'_, Postgres>,
    deltas: &BTreeMap<Uuid, (i32, i32)>,
) -> Result<(), ApiError> {
    for (install, (decided, matching)) in deltas {
        sqlx::query(
            "UPDATE installs SET decided_votes = decided_votes + $2,
                 matching_votes = matching_votes + $3
             WHERE id = $1",
        )
        .bind(install)
        .bind(decided)
        .bind(matching)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// A stable offer ID from a provider and a model name: lowercase ASCII letters and digits,
/// everything else as single dashes, at most 40 characters after the provider.
pub fn offer_id_for(provider: &str, name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug: String = slug
        .trim_end_matches('-')
        .chars()
        .take(MAX_ID_SLUG_CHARS)
        .collect();
    let slug = slug.trim_end_matches('-');
    format!(
        "{provider}.{}",
        if slug.is_empty() { "offer" } else { slug }
    )
}

async fn propose(
    state: &AppState,
    install: &Install,
    provider: &ProviderId,
    name: &str,
    cards: &[RateCard],
    today: Day,
) -> Result<(StatusCode, Json<ReportOutcome>), ApiError> {
    let mut tx = state.db.begin().await?;
    lock_proposals(&mut tx, &provider.0).await?;
    let voter_weight = weight(&install.record, today);
    let matching: Option<ReportRow> = sqlx::query_as(
        "SELECT id, offer_id, proposed_provider, proposed_name, reporter, reporter_weight,
             cards, status
         FROM reports
         WHERE offer_id IS NULL AND status = 'pending' AND proposed_provider = $1
             AND lower(proposed_name) = lower($2) AND cards = $3
         ORDER BY created_on, id LIMIT 1 FOR UPDATE",
    )
    .bind(&provider.0)
    .bind(name)
    .bind(SqlJson(cards))
    .fetch_optional(&mut *tx)
    .await?;
    // The install's own other pending proposal of this name is replaced (not the one it joins).
    // Nobody is settled: the reporter changed their own mind.
    sqlx::query(
        "UPDATE reports SET status = 'superseded', decided_on = $4
         WHERE offer_id IS NULL AND status = 'pending' AND reporter = $1
             AND proposed_provider = $2 AND lower(proposed_name) = lower($3)
             AND id IS DISTINCT FROM $5",
    )
    .bind(install.id)
    .bind(&provider.0)
    .bind(name)
    .bind(today.0)
    .bind(matching.as_ref().map(|report| report.id))
    .execute(&mut *tx)
    .await?;
    if let Some(report) = matching {
        let status = vote_on(&mut tx, &report, install, Answer::Yes, voter_weight, today).await?;
        tx.commit().await?;
        return Ok((
            StatusCode::OK,
            Json(ReportOutcome {
                report: Some(report.id),
                status,
            }),
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO reports (id, proposed_provider, proposed_name, reporter, reporter_weight,
             cards, created_on)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(&provider.0)
    .bind(name)
    .bind(install.id)
    .bind(weight_column(voter_weight))
    .bind(SqlJson(cards))
    .bind(today.0)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(ReportOutcome {
            report: Some(id),
            status: ReportStatus::Pending,
        }),
    ))
}

/// Creates the offer of an accepted proposal, with a free ID.
async fn create_offer(
    tx: &mut Transaction<'_, Postgres>,
    report: &ReportRow,
    today: Day,
) -> Result<String, ApiError> {
    let (Some(provider), Some(name)) = (&report.proposed_provider, &report.proposed_name) else {
        return Err(ApiError::internal("proposal without provider or name"));
    };
    let base = offer_id_for(provider, name);
    for attempt in 1..=100 {
        let id = if attempt == 1 {
            base.clone()
        } else {
            format!("{base}-{attempt}")
        };
        let inserted = sqlx::query(
            "INSERT INTO offers (id, provider_id, name, cards, confirmed_on)
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO NOTHING",
        )
        .bind(&id)
        .bind(provider)
        .bind(name)
        .bind(&report.cards)
        .bind(today.0)
        .execute(&mut **tx)
        .await?
        .rows_affected();
        if inserted == 1 {
            sqlx::query("UPDATE reports SET offer_id = $2 WHERE id = $1")
                .bind(report.id)
                .bind(&id)
                .execute(&mut **tx)
                .await?;
            return Ok(id);
        }
    }
    Err(ApiError::internal("no free offer ID"))
}

fn answer_column(answer: Answer) -> &'static str {
    match answer {
        Answer::Yes => "yes",
        Answer::No => "no",
    }
}

fn weight_column(weight: Weight) -> i32 {
    i32::try_from(weight.0).unwrap_or(i32::MAX)
}

/// The schema's CHECK keeps stored weights within 0–10000.
fn weight_from_column(value: i32) -> Weight {
    Weight::try_from(u32::try_from(value).unwrap_or(0)).unwrap_or(Weight(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_ids_are_lowercase_with_dashes() {
        assert_eq!(offer_id_for("bolt", "Fiat 500e"), "bolt.fiat-500e");
        assert_eq!(offer_id_for("bolt", "  VW  ID.3 "), "bolt.vw-id-3");
        assert_eq!(
            offer_id_for("free2move", "Peugeot e-208"),
            "free2move.peugeot-e-208"
        );
    }

    #[test]
    fn offer_ids_survive_odd_names() {
        assert_eq!(offer_id_for("bolt", "Škoda Enyaq"), "bolt.koda-enyaq");
        assert_eq!(offer_id_for("bolt", "!!!"), "bolt.offer");
        assert_eq!(
            offer_id_for("bolt", &"a".repeat(100)).len(),
            "bolt.".len() + 40
        );
    }
}
