mod common;

use axum::http::StatusCode;
use common::{
    Options, TODAY, TestApp, google_integrity, integrity_token, mock_verdict, registration,
};
use serde_json::json;
use whichride_server::auth;
use whichride_server::integrity::request_hash;
use whichride_server::limits::Kind;
use wiremock::MockServer;

#[tokio::test]
async fn registering_returns_an_id_and_a_token() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (status, body) = app.post("/v1/installs", &registration()).await;
    assert_eq!(status, StatusCode::CREATED);
    let token = body["token"].as_str().unwrap();
    assert_eq!(token.len(), 43);
    let id = body["install_id"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(id).is_ok());

    let install = auth::find_install(&app.pool, token).await.unwrap().unwrap();
    assert_eq!(install.id.to_string(), id);
    assert_eq!(install.record.first_seen.0, TODAY);
    assert_eq!(install.record.decided_votes, 0);
}

#[tokio::test]
async fn only_the_tokens_hash_is_stored() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (_, body) = app.post("/v1/installs", &registration()).await;
    let token = body["token"].as_str().unwrap();
    let stored: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM installs")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(stored, auth::hash_token(token));
    assert_ne!(stored, token.as_bytes());
}

#[tokio::test]
async fn two_registrations_get_different_tokens() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let first = app.register().await;
    let second = app.register().await;
    assert_ne!(first, second);
}

#[tokio::test]
async fn an_unknown_token_finds_nothing() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    assert!(
        auth::find_install(&app.pool, "nope")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn a_body_that_is_not_json_is_a_bad_request() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let request = axum::http::Request::post("/v1/installs")
        .header("content-type", "application/json")
        .body(axum::body::Body::from("not json"))
        .unwrap();
    let (status, body) = app.send(request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, json!({"error": "bad_request"}));
}

#[tokio::test]
async fn registration_needs_a_well_formed_nonce() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    for body in [
        json!({}),
        json!({"nonce": "short"}),
        json!({"nonce": "x".repeat(129)}),
        json!({"nonce": "has spaces in it, sixteen+"}),
        json!({"nonce": "abcdefghijklmnop", "extra": 1}),
        json!({"nonce": 1234567890123456_u64}),
    ] {
        let (status, _) = app.post("/v1/installs", &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    }
    for nonce in ["abcdefghijklmnop".to_owned(), "A-z_0".repeat(25) + "abc"] {
        let (status, _) = app.post("/v1/installs", &json!({"nonce": nonce})).await;
        assert_eq!(status, StatusCode::CREATED, "{nonce}");
    }
}

const NONCE_BODY: &str = r#"{"nonce":"abcdefghijklmnop"}"#;

async fn register_with(app: &TestApp, token: &str) -> StatusCode {
    let request = axum::http::Request::post("/v1/installs")
        .header("content-type", "application/json")
        .header("x-integrity-token", token)
        .body(axum::body::Body::from(NONCE_BODY))
        .unwrap();
    app.send(request).await.0
}

#[tokio::test]
async fn with_google_integrity_registration_needs_a_token_for_this_request() {
    let server = MockServer::start().await;
    let integrity = google_integrity(&server).await;
    let token = integrity_token("registration");
    let hash = request_hash("POST", "/v1/installs", None, NONCE_BODY.as_bytes());
    mock_verdict(&server, &token, &hash).await;
    let Some(app) = TestApp::start_with_integrity(integrity).await else {
        return;
    };

    let (status, body) = app.post("/v1/installs", &registration()).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, json!({"error": "integrity_failed"}));

    assert_eq!(register_with(&app, &token).await, StatusCode::CREATED);
}

#[tokio::test]
async fn a_token_bound_to_another_request_is_refused() {
    let server = MockServer::start().await;
    let integrity = google_integrity(&server).await;
    let token = integrity_token("other");
    mock_verdict(&server, &token, "hash-of-something-else").await;
    let Some(app) = TestApp::start_with_integrity(integrity).await else {
        return;
    };
    assert_eq!(register_with(&app, &token).await, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn a_registration_token_is_accepted_once() {
    let server = MockServer::start().await;
    let integrity = google_integrity(&server).await;
    let token = integrity_token("registration");
    let hash = request_hash("POST", "/v1/installs", None, NONCE_BODY.as_bytes());
    mock_verdict(&server, &token, &hash).await;
    let Some(app) = TestApp::start_with_integrity(integrity).await else {
        return;
    };
    assert_eq!(register_with(&app, &token).await, StatusCode::CREATED);
    assert_eq!(register_with(&app, &token).await, StatusCode::FORBIDDEN);
    let installs: i64 = sqlx::query_scalar("SELECT count(*) FROM installs")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(installs, 1);
}

#[tokio::test]
async fn registrations_are_capped_for_everyone_together() {
    let Some(app) = TestApp::start_with(Options {
        limits: vec![(Kind::Register, 2)],
        ..Options::default()
    })
    .await
    else {
        return;
    };
    app.register().await;
    app.register().await;
    let (status, body) = app.post("/v1/installs", &registration()).await;
    assert_eq!(
        (status, body),
        (
            StatusCode::TOO_MANY_REQUESTS,
            json!({"error": "too_many_requests"})
        )
    );
}

#[tokio::test]
async fn the_registration_cap_is_checked_before_asking_google() {
    let server = MockServer::start().await;
    let integrity = google_integrity(&server).await;
    let first = integrity_token("first");
    let hash = request_hash("POST", "/v1/installs", None, NONCE_BODY.as_bytes());
    mock_verdict(&server, &first, &hash).await;
    let Some(app) = TestApp::start_with(Options {
        integrity,
        limits: vec![(Kind::Register, 1)],
        ..Options::default()
    })
    .await
    else {
        return;
    };
    assert_eq!(register_with(&app, &first).await, StatusCode::CREATED);
    let requests_before = server.received_requests().await.unwrap().len();
    assert_eq!(
        register_with(&app, &integrity_token("second")).await,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        requests_before
    );
}
