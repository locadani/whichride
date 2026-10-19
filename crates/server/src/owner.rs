//! The owner's emergency switch (spec §7.4). Run from a terminal with the production
//! `DATABASE_URL`; there is no HTTP route to any of this.

use std::fmt::Write as _;

use anyhow::{Context, bail};
use carsharing_core::{Day, RateCard};
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

/// `price_history`: cards, from_day, report_id, reverted.
type HistoryRow = (Json<Vec<RateCard>>, i32, Option<Uuid>, bool);
/// A report with its yes and no vote counts.
type ReportSummary = (Uuid, String, i32, Json<Vec<RateCard>>, i64, i64);

pub async fn set_locked(pool: &PgPool, offer: &str, locked: bool) -> anyhow::Result<()> {
    let updated = sqlx::query("UPDATE offers SET locked = $2 WHERE id = $1")
        .bind(offer)
        .bind(locked)
        .execute(pool)
        .await?
        .rows_affected();
    if updated == 0 {
        bail!("no offer {offer:?}");
    }
    Ok(())
}

pub async fn revert_offer(pool: &PgPool, offer: &str, today: Day) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM offers WHERE id = $1 FOR UPDATE")
        .bind(offer)
        .fetch_optional(&mut *tx)
        .await?
        .with_context(|| format!("no offer {offer:?}"))?;
    let history: Vec<(i64, Option<Uuid>, Json<Vec<RateCard>>)> = sqlx::query_as(
        "SELECT seq, report_id, cards FROM price_history
         WHERE offer_id = $1 AND NOT reverted ORDER BY seq DESC LIMIT 2",
    )
    .bind(offer)
    .fetch_all(&mut *tx)
    .await?;
    let [(latest, latest_report, _), (_, _, previous)] = &history[..] else {
        bail!("{offer} has only its first price; nothing to revert");
    };
    sqlx::query("UPDATE price_history SET reverted = true WHERE seq = $1")
        .bind(latest)
        .execute(&mut *tx)
        .await?;
    if let Some(report) = latest_report {
        sqlx::query("UPDATE reports SET status = 'reverted' WHERE id = $1")
            .bind(report)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query(
        "UPDATE reports SET status = 'superseded', decided_on = $2
         WHERE offer_id = $1 AND status = 'pending'",
    )
    .bind(offer)
    .bind(today.0)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE offers SET cards = $2 WHERE id = $1")
        .bind(offer)
        .bind(previous)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn show_offer(pool: &PgPool, offer: &str) -> anyhow::Result<String> {
    let (name, cards, confirmed, locked): (String, Json<Vec<RateCard>>, Option<i32>, bool) =
        sqlx::query_as("SELECT name, cards, confirmed_on, locked FROM offers WHERE id = $1")
            .bind(offer)
            .fetch_optional(pool)
            .await?
            .with_context(|| format!("no offer {offer:?}"))?;
    let mut text = String::new();
    writeln!(
        text,
        "{offer} ({name}){}",
        if locked { " LOCKED" } else { "" }
    )?;
    writeln!(
        text,
        "confirmed: {}",
        confirmed.map_or("never".into(), date)
    )?;
    for card in &cards.0 {
        writeln!(text, "  {}", describe(card))?;
    }
    writeln!(text, "history (newest first):")?;
    let history: Vec<HistoryRow> = sqlx::query_as(
        "SELECT cards, from_day, report_id, reverted FROM price_history WHERE offer_id = $1
         ORDER BY seq DESC LIMIT 10",
    )
    .bind(offer)
    .fetch_all(pool)
    .await?;
    for (cards, day, report, reverted) in history {
        let source = report.map_or("seed".to_owned(), |id| format!("report {id}"));
        let reverted = if reverted { " REVERTED" } else { "" };
        writeln!(text, "  {} from {source}{reverted}", date(day))?;
        for card in &cards.0 {
            writeln!(text, "    {}", describe(card))?;
        }
    }
    writeln!(text, "reports:")?;
    let reports: Vec<ReportSummary> = sqlx::query_as(
        "SELECT r.id, r.status, r.created_on, r.cards,
             count(v.*) FILTER (WHERE v.answer = 'yes'), count(v.*) FILTER (WHERE v.answer = 'no')
         FROM reports r LEFT JOIN votes v ON v.report_id = r.id
         WHERE r.offer_id = $1 GROUP BY r.id ORDER BY r.created_on DESC, r.id LIMIT 20",
    )
    .bind(offer)
    .fetch_all(pool)
    .await?;
    for (id, status, day, cards, yes, no) in reports {
        writeln!(text, "  {id} {status} {} yes {yes} no {no}", date(day))?;
        for card in &cards.0 {
            writeln!(text, "    {}", describe(card))?;
        }
    }
    Ok(text)
}

fn describe(card: &RateCard) -> String {
    format!(
        "{}: base_fee {} per_km {} per_minute {} minimum {} included_km {} included_minutes {}",
        card.label,
        card.base_fee.0,
        card.per_km.0,
        card.per_minute.0,
        card.minimum.0,
        card.included_km,
        card.included_minutes
    )
}

/// `Day` as `YYYY-MM-DD` (civil-from-days, Howard Hinnant's algorithm).
fn date(day: i32) -> String {
    let z = i64::from(day) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_print_as_dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(20_745), "2026-10-19");
        assert_eq!(date(19_782), "2024-02-29");
        assert_eq!(date(-1), "1969-12-31");
    }
}
