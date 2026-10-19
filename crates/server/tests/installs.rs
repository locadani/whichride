mod common;

use axum::http::StatusCode;
use common::{TODAY, TestApp};
use serde_json::json;
use url::Url;
use whichride_server::auth;
use whichride_server::integrity::{GoogleIntegrity, Integrity, request_hash};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn registering_returns_an_id_and_a_token() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (status, body) = app.post("/v1/installs", &json!({})).await;
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
    let (_, body) = app.post("/v1/installs", &json!({})).await;
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

async fn google_integrity(server: &MockServer, request_hash: &str) -> Integrity {
    Mock::given(method("GET"))
        .and(path(
            "/computeMetadata/v1/instance/service-accounts/default/token",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"access_token": "ya29.test", "expires_in": 3599})),
        )
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/app.whichride:decodeIntegrityToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tokenPayloadExternal": {
                "requestDetails": {
                    "requestPackageName": "app.whichride",
                    "requestHash": request_hash,
                    "timestampMillis": whichride_server::integrity::now_millis().to_string()
                },
                "appIntegrity": {"appRecognitionVerdict": "PLAY_RECOGNIZED"},
                "deviceIntegrity": {"deviceRecognitionVerdict": ["MEETS_DEVICE_INTEGRITY"]}
            }
        })))
        .mount(server)
        .await;
    let base = Url::parse(&format!("{}/", server.uri())).unwrap();
    Integrity::Google(GoogleIntegrity::with_endpoints(
        "app.whichride".into(),
        base.clone(),
        base,
    ))
}

#[tokio::test]
async fn with_google_integrity_registration_needs_a_token_for_this_request() {
    let server = MockServer::start().await;
    let hash = request_hash("POST", "/v1/installs", b"{}");
    let integrity = google_integrity(&server, &hash).await;
    let Some(app) = TestApp::start_with_integrity(integrity).await else {
        return;
    };

    let (status, body) = app.post("/v1/installs", &json!({})).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, json!({"error": "integrity_failed"}));

    let (status, _) = app
        .post_with("/v1/installs", &json!({}), &[("x-integrity-token", "tok")])
        .await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn a_token_bound_to_another_request_is_refused() {
    let server = MockServer::start().await;
    let integrity = google_integrity(&server, "hash-of-something-else").await;
    let Some(app) = TestApp::start_with_integrity(integrity).await else {
        return;
    };
    let (status, _) = app
        .post_with("/v1/installs", &json!({}), &[("x-integrity-token", "tok")])
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
