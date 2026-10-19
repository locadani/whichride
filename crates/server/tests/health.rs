use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use carsharing_core::Day;
use tower::ServiceExt;
use whichride_server::app::router;
use whichride_server::state::AppState;

#[tokio::test]
async fn health_answers_ok() {
    let state = AppState {
        today: Arc::new(|| Day(20_745)),
    };
    let response = router(state)
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));
    let body = to_bytes(response.into_body(), 64).await.unwrap();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn unknown_paths_are_json_404s() {
    let state = AppState {
        today: Arc::new(|| Day(20_745)),
    };
    let response = router(state)
        .oneshot(Request::get("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(response.into_body(), 64).await.unwrap();
    assert_eq!(&body[..], br#"{"error":"not_found"}"#);
}
