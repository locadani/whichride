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
async fn a_wrong_method_is_a_json_405() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (status, body) = app.get("/v1/reports").await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(body, serde_json::json!({"error": "method_not_allowed"}));
    let (status, body) = app.post("/v1/catalogue", &serde_json::json!({})).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(body, serde_json::json!({"error": "method_not_allowed"}));
}
