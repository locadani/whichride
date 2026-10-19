mod common;

use carsharing_core::Day;
use common::{TODAY, TestApp, fixture_catalogue};
use whichride_server::db;

#[tokio::test]
async fn seeding_adds_every_offer_with_its_history() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let offers = db::list_offers(&app.pool).await.unwrap();
    let ids: Vec<&str> = offers.iter().map(|offer| offer.id.as_str()).collect();
    assert_eq!(ids, ["bolt.corsa", "miles.l", "miles.s"]);
    assert!(
        offers
            .iter()
            .all(|offer| offer.confirmed_on == Some(TODAY) && !offer.locked)
    );
    let history: i64 = sqlx::query_scalar("SELECT count(*) FROM price_history")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(history, 3);
}

#[tokio::test]
async fn seeding_again_never_overwrites_crowd_prices() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    sqlx::query("UPDATE offers SET cards = '[]'::jsonb WHERE id = 'miles.s'")
        .execute(&app.pool)
        .await
        .unwrap();
    let added = db::seed_offers(&app.pool, &fixture_catalogue(), Day(TODAY + 5))
        .await
        .unwrap();
    assert_eq!(added, 0);
    let cards: serde_json::Value =
        sqlx::query_scalar("SELECT cards FROM offers WHERE id = 'miles.s'")
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(cards, serde_json::json!([]));
}

#[tokio::test]
async fn a_new_seed_offer_is_added_on_the_next_start() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let mut catalogue = fixture_catalogue();
    let mut offer = catalogue.providers[0].offers[0].clone();
    offer.id = carsharing_core::OfferId("miles.xl".into());
    catalogue.providers[0].offers.push(offer);
    let added = db::seed_offers(&app.pool, &catalogue, Day(TODAY + 1))
        .await
        .unwrap();
    assert_eq!(added, 1);
}

#[tokio::test]
async fn migrations_run_twice_without_error() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    db::MIGRATOR.run(&app.pool).await.unwrap();
}

async fn tables(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public'",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn a_placeholder_seed_catalogue_is_refused_before_anything_is_written() {
    let Some(database) = common::fresh_database().await else {
        return;
    };
    let pool = db::connect(&database.url).await.unwrap();

    let mut placeholder = fixture_catalogue();
    placeholder.providers[0].offers[0].id =
        carsharing_core::OfferId("miles.pending-owner-prices".into());
    let error = db::prepare(&pool, &placeholder, Day(TODAY))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("placeholder"), "{error}");

    let mut free = fixture_catalogue();
    let card = &mut free.providers[1].offers[0].cards[0];
    card.base_fee = carsharing_core::Cents(0);
    card.per_km = carsharing_core::Cents(0);
    card.per_minute = carsharing_core::Cents(0);
    let error = db::prepare(&pool, &free, Day(TODAY)).await.unwrap_err();
    assert!(error.to_string().contains("bolt.corsa"), "{error}");

    assert_eq!(tables(&pool).await, 0, "nothing was migrated or seeded");
    db::prepare(&pool, &fixture_catalogue(), Day(TODAY))
        .await
        .unwrap();
    assert!(tables(&pool).await > 0);
    pool.close().await;
}

#[test]
fn the_real_seed_catalogue_is_valid() {
    let catalogue = db::seed_catalogue().expect("crates/server/catalogue.json is valid");
    let providers: Vec<(&str, &str)> = catalogue
        .providers
        .iter()
        .map(|p| (p.id.0.as_str(), p.android_package.as_str()))
        .collect();
    // Spec §5: v1 = MILES, Bolt Drive, Free2move, with these Android packages.
    assert_eq!(
        providers,
        [
            ("miles", "com.driveby.app"),
            ("bolt", "ee.mtakso.client"),
            ("free2move", "com.car2go"),
        ]
    );
    for provider in &catalogue.providers {
        for offer in &provider.offers {
            assert!(
                offer.id.0.starts_with(&format!("{}.", provider.id.0)),
                "offer {} must start with its provider's ID",
                offer.id.0
            );
        }
    }
}

#[test]
#[ignore = "until the owner's prices replace the placeholder catalogue"]
fn the_real_seed_catalogue_has_owner_prices() {
    let catalogue = db::seed_catalogue().expect("crates/server/catalogue.json is valid");
    for provider in &catalogue.providers {
        for offer in &provider.offers {
            assert!(
                !offer.id.0.contains("pending-owner-prices"),
                "offer {} is still a placeholder",
                offer.id.0
            );
            for card in &offer.cards {
                assert!(
                    card.base_fee.0 != 0 || card.per_km.0 != 0 || card.per_minute.0 != 0,
                    "card {:?} of {} has no price",
                    card.label,
                    offer.id.0
                );
            }
        }
    }
}
