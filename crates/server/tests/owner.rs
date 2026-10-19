mod common;

use carsharing_core::Day;
use common::{TODAY, TestApp};
use serde_json::json;
use whichride_server::owner;

fn report(per_km: i64) -> serde_json::Value {
    json!({"offer": "miles.s", "cards": [{
        "label": "Standard", "kind": "standard", "base_fee": 100, "per_km": per_km,
        "per_minute": 0, "minimum": 0,
        "km_rounding": "started_unit", "minute_rounding": "started_unit"
    }]})
}

async fn accepted(app: &TestApp, per_km: i64) {
    let (a, b) = (app.register().await, app.register().await);
    app.post_as(&a, "/v1/reports", &report(per_km)).await;
    let (_, body) = app.post_as(&b, "/v1/reports", &report(per_km)).await;
    assert_eq!(body["status"], "accepted");
}

async fn per_km(app: &TestApp) -> i64 {
    let (_, body) = app.get("/v1/catalogue").await;
    body["catalogue"]["providers"][0]["offers"][1]["cards"][0]["per_km"]
        .as_i64()
        .unwrap()
}

#[tokio::test]
async fn locking_stops_reports_and_unlocking_restarts_them() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    owner::set_locked(&app.pool, "miles.s", true).await.unwrap();
    let a = app.register().await;
    assert_eq!(app.post_as(&a, "/v1/reports", &report(89)).await.0, 409);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["locked"], true);
    owner::set_locked(&app.pool, "miles.s", false)
        .await
        .unwrap();
    assert_eq!(app.post_as(&a, "/v1/reports", &report(89)).await.0, 201);
}

#[tokio::test]
async fn reverting_restores_the_previous_price() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    accepted(&app, 89).await;
    accepted(&app, 300).await; // the abuse
    let c = app.register().await;
    app.post_as(&c, "/v1/reports", &report(301)).await; // pending, part of the attack
    owner::revert_offer(&app.pool, "miles.s", Day(TODAY + 1))
        .await
        .unwrap();
    assert_eq!(per_km(&app).await, 89);
    let statuses: Vec<String> =
        sqlx::query_scalar("SELECT status FROM reports ORDER BY created_on, id")
            .fetch_all(&app.pool)
            .await
            .unwrap();
    assert_eq!(statuses.iter().filter(|s| *s == "reverted").count(), 1);
    assert_eq!(statuses.iter().filter(|s| *s == "pending").count(), 0);
    owner::revert_offer(&app.pool, "miles.s", Day(TODAY + 1))
        .await
        .unwrap();
    assert_eq!(per_km(&app).await, 79); // back to the seed
}

#[tokio::test]
async fn the_first_value_cannot_be_reverted() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    assert!(
        owner::revert_offer(&app.pool, "miles.s", Day(TODAY))
            .await
            .is_err()
    );
    assert!(
        owner::revert_offer(&app.pool, "nope.x", Day(TODAY))
            .await
            .is_err()
    );
    assert!(owner::set_locked(&app.pool, "nope.x", true).await.is_err());
}

#[tokio::test]
async fn show_offer_summarises_prices_and_pending_reports() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    accepted(&app, 89).await;
    let c = app.register().await;
    app.post_as(&c, "/v1/reports", &report(99)).await;
    let text = owner::show_offer(&app.pool, "miles.s").await.unwrap();
    assert!(text.contains("miles.s"), "{text}");
    assert!(text.contains("per_km 89"), "{text}");
    assert!(text.contains("pending"), "{text}");
    assert!(text.contains("per_km 99"), "{text}");
    assert!(
        text.contains("2026-10-19"),
        "dates are shown as dates: {text}"
    );
}
