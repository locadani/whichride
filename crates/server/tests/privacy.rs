mod common;

use axum::http::StatusCode;
use common::{TestApp, capture_logs};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

/// Fragments of the coordinates, the search text and the short link below, as they could
/// appear in any format.
const SECRETS: [&str; 10] = [
    "5219184",
    "52.52",
    "4132147",
    "13.41",
    "5250839",
    "Alexanderplatz",
    "lat_e7",
    "AbC123",
    "goo.gl",
    "10178",
];

const NEAR: &str = r#"{"lat_e7": 525219184, "lon_e7": 134132147}"#;

fn places() -> serde_json::Value {
    json!({"features": [
        {"properties": {"label": "Alexanderplatz, 10178 Berlin, Germany"},
         "geometry": {"coordinates": [13.413_214_7, 52.521_918_4]}}
    ]})
}

async fn search_logs_are_clean(endpoint: &str, upstream: &str) {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    Mock::given(method("GET"))
        .and(path(upstream))
        .respond_with(ResponseTemplate::new(200).set_body_json(places()))
        .expect(1)
        .mount(&app.upstream)
        .await;
    let (logs, _guard) = capture_logs();
    let near: serde_json::Value = serde_json::from_str(NEAR).unwrap();
    let (status, body) = app
        .post_as(
            &token,
            endpoint,
            &json!({"text": "Alexanderplatz", "near": near}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_clean(&logs.lock().unwrap());
}

#[tokio::test]
async fn geocoding_near_the_user_logs_no_location() {
    search_logs_are_clean("/v1/geocode", "/geocode/search").await;
}

#[tokio::test]
async fn autocomplete_near_the_user_logs_no_location() {
    search_logs_are_clean("/v1/autocomplete", "/geocode/autocomplete").await;
}

#[tokio::test]
async fn expanding_a_short_link_logs_neither_the_link_nor_the_place() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    Mock::given(method("GET"))
        .and(path("/AbC123"))
        .respond_with(ResponseTemplate::new(302).insert_header(
            "location",
            "https://www.google.com/maps/place/Alexanderplatz,+10178+Berlin/@52.52,13.41,15z/data=!3m1!4b1!4m6!3m5!1s0x0:0x0!8m2!3d52.5219184!4d13.4132147",
        ))
        .mount(&app.upstream)
        .await;
    let (logs, _guard) = capture_logs();
    let (status, body) = app
        .post_as(
            &token,
            "/v1/expand",
            &json!({"url": "https://maps.app.goo.gl/AbC123"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // And a failing expansion logs nothing of it either.
    app.post_as(
        &token,
        "/v1/expand",
        &json!({"url": "https://maps.app.goo.gl/AbC123Unknown"}),
    )
    .await;
    assert_clean(&logs.lock().unwrap());
}

fn assert_clean(logs: &[u8]) {
    let logs = String::from_utf8_lossy(logs);
    assert!(
        logs.contains("/v1/"),
        "the capture works and logs routes:\n{logs}"
    );
    for secret in SECRETS {
        assert!(!logs.contains(secret), "logs contain {secret:?}:\n{logs}");
    }
}

#[tokio::test]
async fn routing_logs_contain_no_location() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    Mock::given(method("POST"))
        .and(path("/v2/directions/driving-car"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(
                json!({"routes": [{"summary": {"distance": 1.0, "duration": 1.0}}]}),
            ),
        )
        .mount(&app.upstream)
        .await;
    let (logs, _guard) = capture_logs();
    let body = json!({
        "from": {"lat_e7": 525_219_184, "lon_e7": 134_132_147},
        "to": {"lat_e7": 525_250_839, "lon_e7": 133_694_195}
    });
    assert_eq!(
        app.post_as(&token, "/v1/route", &body).await.0,
        StatusCode::OK
    );
    assert_clean(&logs.lock().unwrap());
}

#[tokio::test]
async fn failures_log_no_location_either() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("at 52.5219184,13.4132147"))
        .mount(&app.upstream)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Alexanderplatz failed"))
        .mount(&app.upstream)
        .await;
    let (logs, _guard) = capture_logs();
    let route = json!({
        "from": {"lat_e7": 525_219_184, "lon_e7": 134_132_147},
        "to": {"lat_e7": 525_250_839, "lon_e7": 133_694_195}
    });
    app.post_as(&token, "/v1/route", &route).await;
    app.post_as(&token, "/v1/geocode", &json!({"text": "Alexanderplatz"}))
        .await;
    // A path a client made up is logged as "unmatched", not as written.
    app.get("/v1/route/52.5219184,13.4132147").await;
    assert_clean(&logs.lock().unwrap());
}
