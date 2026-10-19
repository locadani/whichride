mod common;

use axum::http::StatusCode;
use common::{TODAY, TestApp};
use serde_json::json;

#[tokio::test]
async fn the_catalogue_combines_seed_providers_and_database_offers() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (status, body) = app.get("/v1/catalogue").await;
    assert_eq!(status, StatusCode::OK);
    let providers = body["catalogue"]["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 2);
    assert_eq!(providers[0]["id"], "miles");
    assert_eq!(providers[0]["android_package"], "com.driveby.app");
    let miles_offers: Vec<&str> = providers[0]["offers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|offer| offer["id"].as_str().unwrap())
        .collect();
    assert_eq!(miles_offers, ["miles.l", "miles.s"]);
    assert_eq!(
        body["offers"][0],
        json!({"offer": "miles.l", "confirmed_on": TODAY, "locked": false, "pending": null})
    );
    assert_eq!(body["proposals"], json!([]));
}

#[tokio::test]
async fn the_catalogue_serves_crowd_prices_not_seed_prices() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    sqlx::query(
        "UPDATE offers SET cards = jsonb_set(cards, '{0,per_km}', '99') WHERE id = 'miles.s'",
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let (_, body) = app.get("/v1/catalogue").await;
    assert_eq!(
        body["catalogue"]["providers"][0]["offers"][1]["cards"][0]["per_km"],
        99
    );
}

#[tokio::test]
async fn the_catalogue_parses_with_the_core() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (_, body) = app.get("/v1/catalogue").await;
    let catalogue: carsharing_core::Catalogue =
        serde_json::from_value(body["catalogue"].clone()).unwrap();
    catalogue.validate().unwrap();
}

#[tokio::test]
async fn offers_of_a_provider_no_longer_in_the_seed_are_hidden() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    sqlx::query(
        "INSERT INTO offers (id, provider_id, name, cards) SELECT 'gone.x', 'gone', 'X', cards
         FROM offers WHERE id = 'miles.s'",
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let (_, body) = app.get("/v1/catalogue").await;
    assert!(!body.to_string().contains("gone.x"));
}
