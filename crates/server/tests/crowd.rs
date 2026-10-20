mod common;

use axum::http::StatusCode;
use common::{Options, TODAY, TestApp, google_integrity, integrity_token, mock_verdict};
use serde_json::{Value, json};
use url::Url;
use whichride_server::integrity::{GoogleIntegrity, Integrity, request_hash};
use whichride_server::limits::Kind;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The fixture's `miles.s` card with another per-km price.
fn card(per_km: i64) -> Value {
    json!({
        "label": "Standard", "kind": "standard", "base_fee": 100, "per_km": per_km,
        "per_minute": 0, "minimum": 0,
        "km_rounding": "started_unit", "minute_rounding": "started_unit"
    })
}

fn report(per_km: i64) -> Value {
    json!({"offer": "miles.s", "cards": [card(per_km)]})
}

/// Labels and names with control or bidirectional-formatting characters.
const UNSAFE_TEXT: [&str; 10] = [
    "Stan\u{0}dard",
    "Stan\ndard",
    "Stan\tdard",
    "Stan\u{7f}dard",
    "Stan\u{85}dard",
    "Stan\u{200e}dard",
    "Stan\u{200f}dard",
    "Stan\u{202a}dard",
    "Stan\u{202e}dard",
    "Stan\u{2069}dard",
];

async fn per_km(app: &TestApp) -> i64 {
    let (_, body) = app.get("/v1/catalogue").await;
    body["catalogue"]["providers"][0]["offers"][1]["cards"][0]["per_km"]
        .as_i64()
        .unwrap()
}

async fn record(app: &TestApp, token: &str) -> (i32, i32) {
    sqlx::query_as("SELECT decided_votes, matching_votes FROM installs WHERE token_hash = $1")
        .bind(whichride_server::auth::hash_token(token).as_slice())
        .fetch_one(&app.pool)
        .await
        .unwrap()
}

async fn history_rows(app: &TestApp) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM price_history WHERE offer_id = 'miles.s'")
        .fetch_one(&app.pool)
        .await
        .unwrap()
}

async fn vote(app: &TestApp, token: &str, report: &str, answer: &str) -> (StatusCode, Value) {
    app.post_as(
        token,
        &format!("/v1/reports/{report}/votes"),
        &json!({"answer": answer}),
    )
    .await
}

#[tokio::test]
async fn reports_need_a_registered_install() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (status, _) = app.post("/v1/reports", &report(89)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = app.post_as("made-up", "/v1/reports", &report(89)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_report_alone_changes_nothing_for_others() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    let (status, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "pending");
    assert_eq!(per_km(&app).await, 79);
    // The confirmation question shows the pending values (decision 37).
    let (_, catalogue) = app.get("/v1/catalogue").await;
    let status = &catalogue["offers"][1];
    assert_eq!(status["offer"], "miles.s");
    assert_eq!(status["pending"]["report"], body["report"]);
    assert_eq!(status["pending"]["cards"][0]["per_km"], 89);
}

#[tokio::test]
async fn one_confirmation_from_another_install_accepts_the_report() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (a, b) = (app.register().await, app.register().await);
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap();
    app.set_today(TODAY + 3);
    let (status, body) = vote(&app, &b, id, "yes").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"status": "accepted"}));
    assert_eq!(per_km(&app).await, 89);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["confirmed_on"], TODAY + 3);
    assert_eq!(catalogue["offers"][1]["pending"], Value::Null);
    assert_eq!(history_rows(&app).await, 2);
    // Both were right.
    assert_eq!(record(&app, &a).await, (1, 1));
    assert_eq!(record(&app, &b).await, (1, 1));
}

#[tokio::test]
async fn weighted_no_votes_keep_the_report_pending() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (a, b, c) = (
        app.register().await,
        app.register().await,
        app.register().await,
    );
    app.make_trusted(&b, 8, 8, 80).await; // weight 9000
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap();
    assert_eq!(vote(&app, &b, id, "no").await.1["status"], "pending");
    // 1000 (reporter) + 1000 (c) is not more than 9000.
    assert_eq!(vote(&app, &c, id, "yes").await.1["status"], "pending");
    assert_eq!(per_km(&app).await, 79);
}

#[tokio::test]
async fn the_reporter_cannot_confirm_their_own_report() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap();
    for _ in 0..3 {
        assert_eq!(vote(&app, &a, id, "yes").await.1["status"], "pending");
    }
    assert_eq!(per_km(&app).await, 79);
}

#[tokio::test]
async fn an_identical_report_counts_as_a_yes_vote() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (a, b) = (app.register().await, app.register().await);
    let (_, first) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let (status, second) = app.post_as(&b, "/v1/reports", &report(89)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["report"], first["report"]);
    assert_eq!(second["status"], "accepted");
    assert_eq!(per_km(&app).await, 89);
}

#[tokio::test]
async fn reporting_the_current_price_confirms_it() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    app.set_today(TODAY + 10);
    let (status, body) = app.post_as(&a, "/v1/reports", &report(79)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"report": null, "status": "current"}));
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["confirmed_on"], TODAY + 10);
}

#[tokio::test]
async fn reporting_the_current_price_replaces_the_installs_pending_report() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (a, b) = (app.register().await, app.register().await);
    let (_, mine) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let (status, body) = app.post_as(&a, "/v1/reports", &report(79)).await;
    assert_eq!(
        (status, &body["status"]),
        (StatusCode::OK, &json!("current"))
    );
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["pending"], Value::Null);
    let (status, _) = vote(&app, &b, mine["report"].as_str().unwrap(), "yes").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(record(&app, &a).await, (0, 0), "nobody is settled");
}

#[tokio::test]
async fn a_new_report_replaces_the_same_installs_pending_one() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    let (_, first) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let (_, second) = app.post_as(&a, "/v1/reports", &report(99)).await;
    assert_ne!(first["report"], second["report"]);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(
        catalogue["offers"][1]["pending"]["report"],
        second["report"]
    );
    let b = app.register().await;
    let (status, body) = vote(&app, &b, first["report"].as_str().unwrap(), "yes").await;
    assert_eq!(
        (status, body),
        (StatusCode::CONFLICT, json!({"error": "report_closed"}))
    );
}

#[tokio::test]
async fn accepting_a_report_supersedes_the_others_and_settles_everyone() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (a, b, c, d) = (
        app.register().await,
        app.register().await,
        app.register().await,
        app.register().await,
    );
    let (_, x) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let (_, y) = app.post_as(&c, "/v1/reports", &report(99)).await;
    let (x, y) = (x["report"].as_str().unwrap(), y["report"].as_str().unwrap());
    vote(&app, &d, y, "no").await;
    assert_eq!(vote(&app, &b, x, "yes").await.1["status"], "accepted");
    assert_eq!(vote(&app, &d, y, "yes").await.0, StatusCode::CONFLICT);
    assert_eq!(record(&app, &a).await, (1, 1));
    assert_eq!(record(&app, &b).await, (1, 1));
    assert_eq!(record(&app, &c).await, (1, 0)); // its report lost
    assert_eq!(record(&app, &d).await, (1, 1)); // said no to the loser
}

#[tokio::test]
async fn a_locked_offer_takes_no_reports_or_votes() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (a, b) = (app.register().await, app.register().await);
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap();
    sqlx::query("UPDATE offers SET locked = true WHERE id = 'miles.s'")
        .execute(&app.pool)
        .await
        .unwrap();
    let locked = (StatusCode::CONFLICT, json!({"error": "offer_locked"}));
    assert_eq!(app.post_as(&b, "/v1/reports", &report(99)).await, locked);
    assert_eq!(vote(&app, &b, id, "yes").await, locked);
    assert_eq!(per_km(&app).await, 79);
}

#[tokio::test]
async fn implausible_reports_are_rejected() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    let long_label = {
        let mut card = card(89);
        card["label"] = "x".repeat(41).into();
        card
    };
    let empty_label = {
        let mut card = card(89);
        card["label"] = "".into();
        card
    };
    let padded_label = {
        let mut card = card(89);
        card["label"] = "Standard ".into();
        card
    };
    let package = {
        let mut card = card(89);
        card["kind"] = "package".into();
        card["base_fee"] = 50_001.into();
        card
    };
    for body in [
        report(100_000),
        report(-1),
        json!({"offer": "miles.s", "cards": []}),
        json!({"offer": "miles.s", "cards": vec![card(89); 6]}),
        json!({"offer": "miles.s", "cards": [long_label]}),
        json!({"offer": "miles.s", "cards": [empty_label]}),
        json!({"offer": "miles.s", "cards": [padded_label]}),
        json!({"offer": "miles.s", "cards": [package]}),
        json!({"offer": "miles.s", "cards": [card(89)], "extra": 1}),
        json!({"offer": "miles.s", "provider": "miles", "name": "X", "cards": [card(89)]}),
        json!({"cards": [card(89)]}),
    ] {
        let (status, response) = app.post_as(&a, "/v1/reports", &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body} → {response}");
    }
    for label in UNSAFE_TEXT {
        let mut card = card(89);
        card["label"] = label.into();
        let body = json!({"offer": "miles.s", "cards": [card]});
        let (status, _) = app.post_as(&a, "/v1/reports", &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{label:?}");
    }
    let (status, _) = app
        .post_as(
            &a,
            "/v1/reports",
            &json!({"offer": "nope.x", "cards": [card(89)]}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let reports: i64 = sqlx::query_scalar("SELECT count(*) FROM reports")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(reports, 0);
}

#[tokio::test]
async fn votes_on_unknown_reports_are_not_found() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    let unknown = uuid::Uuid::new_v4().to_string();
    assert_eq!(
        vote(&app, &a, &unknown, "yes").await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        vote(&app, &a, "not-a-uuid", "yes").await.0,
        StatusCode::NOT_FOUND
    );
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap();
    assert_eq!(vote(&app, &a, id, "maybe").await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_confirmations_accept_once() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    let mut voters = Vec::new();
    for _ in 0..4 {
        voters.push(app.register().await);
    }
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap().to_owned();
    let results = futures_join_all(&app, &voters, &id).await;
    let accepted = results.iter().filter(|s| **s == StatusCode::OK).count();
    let closed = results
        .iter()
        .filter(|s| **s == StatusCode::CONFLICT)
        .count();
    assert_eq!((accepted, closed), (1, 3), "{results:?}");
    assert_eq!(history_rows(&app).await, 2);
    assert_eq!(record(&app, &a).await, (1, 1));
}

/// Sends every voter's yes at the same time.
async fn futures_join_all(app: &TestApp, voters: &[String], id: &str) -> Vec<StatusCode> {
    let mut handles = Vec::new();
    for token in voters {
        let router = app.router.clone();
        let request = axum::http::Request::post(format!("/v1/reports/{id}/votes"))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(axum::body::Body::from(r#"{"answer":"yes"}"#))
            .unwrap();
        handles.push(tokio::spawn(async move {
            use tower::ServiceExt;
            router.oneshot(request).await.unwrap().status()
        }));
    }
    let mut statuses = Vec::new();
    for handle in handles {
        statuses.push(handle.await.unwrap());
    }
    statuses
}

#[tokio::test]
async fn confirming_the_current_price_refreshes_its_date() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    app.set_today(TODAY + 20);
    let path = "/v1/offers/miles.s/confirmations";
    let (status, _) = app.post_as(&a, path, &json!({"answer": "no"})).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["confirmed_on"], TODAY); // decision 36
    let (status, _) = app.post_as(&a, path, &json!({"answer": "yes"})).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["confirmed_on"], TODAY + 20);
}

#[tokio::test]
async fn a_confirmation_without_weight_changes_nothing() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    app.make_trusted(&a, 1_000, 0, 0).await; // weight 0
    app.set_today(TODAY + 20);
    app.post_as(
        &a,
        "/v1/offers/miles.s/confirmations",
        &json!({"answer": "yes"}),
    )
    .await;
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["confirmed_on"], TODAY);
}

#[tokio::test]
async fn confirming_an_unknown_offer_is_not_found() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let a = app.register().await;
    let (status, _) = app
        .post_as(
            &a,
            "/v1/offers/nope.x/confirmations",
            &json!({"answer": "yes"}),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn checks_run_bearer_then_body_then_integrity() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/app.whichride:decodeIntegrityToken"))
        .respond_with(ResponseTemplate::new(500))
        .expect(0)
        .mount(&server)
        .await;
    let base = Url::parse(&format!("{}/", server.uri())).unwrap();
    let integrity = Integrity::Google(GoogleIntegrity::with_endpoints(
        "app.whichride".into(),
        base.clone(),
        base,
    ));
    let Some(app) = TestApp::start_with_integrity(integrity).await else {
        return;
    };
    sqlx::query("INSERT INTO installs (id, token_hash, first_seen) VALUES ($1, $2, $3)")
        .bind(uuid::Uuid::new_v4())
        .bind(whichride_server::auth::hash_token("known").as_slice())
        .bind(TODAY)
        .execute(&app.pool)
        .await
        .unwrap();

    let send = |bearer: Option<&str>| {
        let mut request = axum::http::Request::post("/v1/reports")
            .header("content-type", "application/json")
            .header("x-integrity-token", "dummy");
        if let Some(token) = bearer {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        app.send(request.body(axum::body::Body::from("not json")).unwrap())
    };
    assert_eq!(send(None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(send(Some("known")).await.0, StatusCode::BAD_REQUEST);
    server.verify().await;
}

#[tokio::test]
async fn joining_another_report_supersedes_the_installs_own_and_settles_it_once() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (a, b) = (app.register().await, app.register().await);
    let (_, mine) = app.post_as(&a, "/v1/reports", &report(99)).await;
    app.post_as(&b, "/v1/reports", &report(89)).await;
    let (status, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    assert_eq!(
        (status, &body["status"]),
        (StatusCode::OK, &json!("accepted"))
    );
    let (status, body) = vote(&app, &b, mine["report"].as_str().unwrap(), "yes").await;
    assert_eq!(
        (status, body),
        (StatusCode::CONFLICT, json!({"error": "report_closed"}))
    );
    assert_eq!(per_km(&app).await, 89);
    assert_eq!(record(&app, &a).await, (1, 1));
    assert_eq!(record(&app, &b).await, (1, 1));
}

const CONFIRM: &str = "/v1/offers/miles.s/confirmations";
const YES: &str = r#"{"answer":"yes"}"#;

async fn confirm_with(app: &TestApp, bearer: &str, token: &str) -> StatusCode {
    let request = axum::http::Request::post(CONFIRM)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {bearer}"))
        .header("x-integrity-token", token)
        .body(axum::body::Body::from(YES))
        .unwrap();
    app.send(request).await.0
}

#[tokio::test]
async fn an_integrity_token_is_good_for_one_install_and_one_request() {
    let server = MockServer::start().await;
    let integrity = google_integrity(&server).await;
    let Some(app) = TestApp::start_with_integrity(integrity).await else {
        return;
    };
    let a = app.insert_install("bearer-a").await;
    app.insert_install("bearer-b").await;
    let token = integrity_token("for-a");
    let hash = request_hash("POST", CONFIRM, Some(a), YES.as_bytes());
    mock_verdict(&server, &token, &hash).await;

    // Another install sending the same vote with A's token is refused.
    assert_eq!(
        confirm_with(&app, "bearer-b", &token).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        confirm_with(&app, "bearer-a", &token).await,
        StatusCode::NO_CONTENT
    );
    // And A cannot use it twice.
    assert_eq!(
        confirm_with(&app, "bearer-a", &token).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn a_signed_request_hash_must_include_the_install() {
    let server = MockServer::start().await;
    let integrity = google_integrity(&server).await;
    let Some(app) = TestApp::start_with_integrity(integrity).await else {
        return;
    };
    app.insert_install("bearer-a").await;
    let token = integrity_token("unbound");
    let hash = request_hash("POST", CONFIRM, None, YES.as_bytes());
    mock_verdict(&server, &token, &hash).await;
    assert_eq!(
        confirm_with(&app, "bearer-a", &token).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn writes_have_an_hourly_limit_per_install_checked_before_asking_google() {
    let server = MockServer::start().await;
    let integrity = google_integrity(&server).await;
    let Some(app) = TestApp::start_with(Options {
        integrity,
        limits: vec![(Kind::Write, 1)],
        ..Options::default()
    })
    .await
    else {
        return;
    };
    let a = app.insert_install("bearer-a").await;
    app.insert_install("bearer-b").await;
    let first = integrity_token("first");
    mock_verdict(
        &server,
        &first,
        &request_hash("POST", CONFIRM, Some(a), YES.as_bytes()),
    )
    .await;
    assert_eq!(
        confirm_with(&app, "bearer-a", &first).await,
        StatusCode::NO_CONTENT
    );
    let requests_before = server.received_requests().await.unwrap().len();
    assert_eq!(
        confirm_with(&app, "bearer-a", &integrity_token("second")).await,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        requests_before
    );
    // Another install has its own allowance.
    assert_eq!(
        confirm_with(&app, "bearer-b", &integrity_token("third")).await,
        StatusCode::FORBIDDEN
    );
}
