mod common;

use axum::http::StatusCode;
use common::{TODAY, TestApp};
use serde_json::{Value, json};

fn card() -> Value {
    json!({
        "label": "Standard", "kind": "standard", "base_fee": 0, "per_km": 20,
        "per_minute": 22, "minimum": 300,
        "km_rounding": "started_unit", "minute_rounding": "started_unit"
    })
}

fn proposal(name: &str) -> Value {
    json!({"provider": "bolt", "name": name, "cards": [card()]})
}

async fn bolt_offers(app: &TestApp) -> Vec<(String, String)> {
    let (_, body) = app.get("/v1/catalogue").await;
    body["catalogue"]["providers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "bolt")
        .unwrap()["offers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| {
            (
                o["id"].as_str().unwrap().into(),
                o["name"].as_str().unwrap().into(),
            )
        })
        .collect()
}

#[tokio::test]
async fn a_proposal_is_listed_but_not_offered_until_confirmed() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    let (status, body) = app.post_as(&a, "/v1/reports", &proposal("Fiat 500")).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "pending");
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["proposals"][0]["report"], body["report"]);
    assert_eq!(catalogue["proposals"][0]["provider"], "bolt");
    assert_eq!(catalogue["proposals"][0]["name"], "Fiat 500");
    assert_eq!(bolt_offers(&app).await.len(), 1);
}

#[tokio::test]
async fn a_confirmed_proposal_becomes_an_offer() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (a, b) = (app.register().await, app.register().await);
    let (_, body) = app.post_as(&a, "/v1/reports", &proposal("Fiat 500")).await;
    let id = body["report"].as_str().unwrap();
    let (_, vote) = app
        .post_as(
            &b,
            &format!("/v1/reports/{id}/votes"),
            &json!({"answer": "yes"}),
        )
        .await;
    assert_eq!(vote["status"], "accepted");
    assert!(
        bolt_offers(&app)
            .await
            .contains(&("bolt.fiat-500".into(), "Fiat 500".into()))
    );
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["proposals"], json!([]));
    let offer = catalogue["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["offer"] == "bolt.fiat-500")
        .unwrap();
    assert_eq!(offer["confirmed_on"], TODAY);
}

#[tokio::test]
async fn the_same_proposal_from_another_install_confirms_it() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (a, b) = (app.register().await, app.register().await);
    app.post_as(&a, "/v1/reports", &proposal("Fiat 500")).await;
    let (status, body) = app
        .post_as(&b, "/v1/reports", &proposal("  fiat 500 "))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "accepted");
}

#[tokio::test]
async fn a_proposal_named_like_an_existing_offer_reports_on_it() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    let (status, _) = app
        .post_as(&a, "/v1/reports", &proposal("opel corsa"))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["proposals"], json!([]));
    let corsa = catalogue["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["offer"] == "bolt.corsa")
        .unwrap()
        .clone();
    assert_eq!(corsa["pending"]["cards"][0]["per_km"], 20);
}

#[tokio::test]
async fn a_taken_offer_id_gets_a_number() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    sqlx::query(
        "INSERT INTO offers (id, provider_id, name, cards)
         SELECT 'bolt.fiat-500', 'bolt', 'Fiat 500 (old)', cards FROM offers WHERE id = 'bolt.corsa'",
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let (a, b) = (app.register().await, app.register().await);
    app.post_as(&a, "/v1/reports", &proposal("Fiat 500")).await;
    app.post_as(&b, "/v1/reports", &proposal("Fiat 500")).await;
    assert!(
        bolt_offers(&app)
            .await
            .contains(&("bolt.fiat-500-2".into(), "Fiat 500".into()))
    );
}

#[tokio::test]
async fn bad_proposals_are_rejected() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    let unknown = json!({"provider": "nope", "name": "X", "cards": [card()]});
    assert_eq!(
        app.post_as(&a, "/v1/reports", &unknown).await.0,
        StatusCode::NOT_FOUND
    );
    for name in ["", "   ", &"x".repeat(61)] {
        let (status, _) = app.post_as(&a, "/v1/reports", &proposal(name)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name:?}");
    }
    let no_name = json!({"provider": "bolt", "cards": [card()]});
    assert_eq!(
        app.post_as(&a, "/v1/reports", &no_name).await.0,
        StatusCode::BAD_REQUEST
    );
}
