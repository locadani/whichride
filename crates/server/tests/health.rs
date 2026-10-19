mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::TestApp;
use tower::ServiceExt;

#[tokio::test]
async fn health_answers_ok() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let response = app
        .router
        .clone()
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test]
async fn unknown_paths_are_json_404s() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (status, body) = app.get("/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, serde_json::json!({"error": "not_found"}));
}

#[tokio::test]
async fn a_request_id_from_the_client_is_replaced() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (logs, _guard) = common::capture_logs();
    let response = app
        .router
        .clone()
        .oneshot(
            Request::get("/v1/catalogue")
                .header("x-request-id", "52.52,13.41")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let id = response.headers()["x-request-id"].to_str().unwrap();
    assert_ne!(id, "52.52,13.41");
    assert!(uuid::Uuid::parse_str(id).is_ok(), "{id}");
    let logs = String::from_utf8_lossy(&logs.lock().unwrap()).into_owned();
    assert!(logs.contains(id), "the new ID is logged:\n{logs}");
    assert!(!logs.contains("52.52"), "{logs}");
}
