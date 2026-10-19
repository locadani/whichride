//! The Postgres connection, migrations and the seed catalogue.

use carsharing_core::{Catalogue, CatalogueError, Day, RateCard};
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Json;
use sqlx::{FromRow, PgPool};

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Providers and starting prices, edited by pull request (spec §5.1).
const SEED_CATALOGUE: &str = include_str!("../catalogue.json");

pub fn seed_catalogue() -> Result<Catalogue, CatalogueError> {
    Catalogue::from_json(SEED_CATALOGUE)
}

/// A small pool: one Cloud Run instance, and Neon's free plan limits connections.
pub async fn connect(url: &str) -> sqlx::Result<PgPool> {
    PgPoolOptions::new().max_connections(5).connect(url).await
}

/// Migrates, then adds the seed catalogue's new offers.
pub async fn prepare(pool: &PgPool, seed: &Catalogue, today: Day) -> anyhow::Result<()> {
    MIGRATOR.run(pool).await?;
    let added = seed_offers(pool, seed, today).await?;
    if added > 0 {
        tracing::info!(added, "offers added from the seed catalogue");
    }
    Ok(())
}

/// Inserts every seed offer the database does not have yet, confirmed today (the owner just
/// read the prices from the apps). Existing offers keep their crowd prices.
pub async fn seed_offers(pool: &PgPool, seed: &Catalogue, today: Day) -> sqlx::Result<u64> {
    let mut tx = pool.begin().await?;
    let mut added = 0;
    for provider in &seed.providers {
        for offer in &provider.offers {
            let inserted = sqlx::query(
                "INSERT INTO offers (id, provider_id, name, cards, confirmed_on)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (id) DO NOTHING",
            )
            .bind(&offer.id.0)
            .bind(&provider.id.0)
            .bind(&offer.name)
            .bind(Json(&offer.cards))
            .bind(today.0)
            .execute(&mut *tx)
            .await?
            .rows_affected();
            if inserted == 1 {
                sqlx::query(
                    "INSERT INTO price_history (offer_id, cards, from_day) VALUES ($1, $2, $3)",
                )
                .bind(&offer.id.0)
                .bind(Json(&offer.cards))
                .bind(today.0)
                .execute(&mut *tx)
                .await?;
                added += 1;
            }
        }
    }
    tx.commit().await?;
    Ok(added)
}

#[derive(Debug, Clone, FromRow)]
pub struct OfferRow {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub cards: Json<Vec<RateCard>>,
    pub confirmed_on: Option<i32>,
    pub locked: bool,
}

/// Every offer the owner has not hidden.
pub async fn list_offers(pool: &PgPool) -> sqlx::Result<Vec<OfferRow>> {
    sqlx::query_as(
        "SELECT id, provider_id, name, cards, confirmed_on, locked FROM offers
         WHERE NOT hidden ORDER BY id",
    )
    .fetch_all(pool)
    .await
}
