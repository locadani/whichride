mod common;

use axum::http::StatusCode;
use common::{Options, TestApp};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

const ALEXANDERPLATZ: (i32, i32) = (525_219_184, 134_132_147);
const HAUPTBAHNHOF: (i32, i32) = (525_250_839, 133_694_195);

fn route_body() -> serde_json::Value {
    json!({
        "from": {"lat_e7": ALEXANDERPLATZ.0, "lon_e7": ALEXANDERPLATZ.1},
        "to": {"lat_e7": HAUPTBAHNHOF.0, "lon_e7": HAUPTBAHNHOF.1}
    })
}

async fn mock_route(app: &TestApp, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/v2/directions/driving-car"))
        .and(header("authorization", "test-key"))
        .and(body_json(
            json!({"coordinates": [[13.413_214_7, 52.521_918_4], [13.369_419_5, 52.525_083_9]]}),
        ))
        .respond_with(response)
        .mount(&app.upstream)
        .await;
}

#[tokio::test]
async fn a_route_returns_metres_and_seconds() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    mock_route(
        &app,
        ResponseTemplate::new(200).set_body_json(
            json!({"routes": [{"summary": {"distance": 4_321.6, "duration": 612.4}}]}),
        ),
    )
    .await;
    let (status, body) = app.post_as(&token, "/v1/route", &route_body()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({"metres": 4_322, "seconds": 612}));
}

#[tokio::test]
async fn a_route_needs_a_registered_install() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (status, _) = app.post("/v1/route", &route_body()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn upstream_trouble_means_route_service_busy() {
    for upstream in [
        ResponseTemplate::new(429),
        ResponseTemplate::new(403).set_body_string("Daily quota reached"),
        ResponseTemplate::new(500).set_body_string("<html>stack trace</html>"),
        ResponseTemplate::new(200).set_body_string("not json"),
        ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(2)),
    ] {
        let Some(app) = TestApp::start().await else {
            return;
        };
        let token = app.register().await;
        mock_route(&app, upstream).await;
        let (status, body) = app.post_as(&token, "/v1/route", &route_body()).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body, json!({"error": "route_service_busy"}));
    }
}

#[tokio::test]
async fn no_route_is_its_own_error() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    mock_route(&app, ResponseTemplate::new(404)).await;
    let (status, body) = app.post_as(&token, "/v1/route", &route_body()).await;
    assert_eq!(
        (status, body),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({"error": "no_route"})
        )
    );
}

#[tokio::test]
async fn coordinates_off_the_earth_never_reach_the_routing_service() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.upstream)
        .await;
    let body = json!({
        "from": {"lat_e7": 900_000_001, "lon_e7": 0},
        "to": {"lat_e7": 0, "lon_e7": 0}
    });
    assert_eq!(
        app.post_as(&token, "/v1/route", &body).await.0,
        StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn each_install_has_an_hourly_limit() {
    let Some(app) = TestApp::start_with(Options {
        per_hour: Some(2),
        ..Options::default()
    })
    .await
    else {
        return;
    };
    let token = app.register().await;
    mock_route(
        &app,
        ResponseTemplate::new(200).set_body_json(json!({"routes": [{"summary": {}}]})),
    )
    .await;
    assert_eq!(
        app.post_as(&token, "/v1/route", &route_body()).await.1,
        json!({"metres": 0, "seconds": 0})
    );
    assert_eq!(
        app.post_as(&token, "/v1/route", &route_body()).await.0,
        StatusCode::OK
    );
    let (status, body) = app.post_as(&token, "/v1/route", &route_body()).await;
    assert_eq!(
        (status, body),
        (
            StatusCode::TOO_MANY_REQUESTS,
            json!({"error": "too_many_requests"})
        )
    );
    let other = app.register().await;
    assert_eq!(
        app.post_as(&other, "/v1/route", &route_body()).await.0,
        StatusCode::OK
    );
}

fn features() -> serde_json::Value {
    json!({"features": [
        {"properties": {"label": "Alexanderplatz, Berlin, Germany"},
         "geometry": {"coordinates": [13.413_214_7, 52.521_918_4]}},
        {"properties": {"label": "Broken"}, "geometry": {"coordinates": [999.0, 52.0]}},
        {"properties": {}, "geometry": {"coordinates": [13.0, 52.0]}}
    ]})
}

#[tokio::test]
async fn geocoding_returns_places_near_the_user() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    Mock::given(method("GET"))
        .and(path("/geocode/search"))
        .and(header("authorization", "test-key"))
        .and(query_param("text", "Alexanderplatz"))
        .and(query_param("boundary.country", "DE"))
        .and(query_param("focus.point.lat", "52.5"))
        .and(query_param("focus.point.lon", "13.4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(features()))
        .mount(&app.upstream)
        .await;
    let body =
        json!({"text": " Alexanderplatz ", "near": {"lat_e7": 525_000_000, "lon_e7": 134_000_000}});
    let (status, places) = app.post_as(&token, "/v1/geocode", &body).await;
    assert_eq!(status, StatusCode::OK, "{places}");
    // Places with impossible coordinates or no label are dropped.
    assert_eq!(
        places,
        json!({"places": [{"label": "Alexanderplatz, Berlin, Germany",
                           "at": {"lat_e7": ALEXANDERPLATZ.0, "lon_e7": ALEXANDERPLATZ.1}}]})
    );
}

#[tokio::test]
async fn autocomplete_uses_its_own_endpoint() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    Mock::given(method("GET"))
        .and(path("/geocode/autocomplete"))
        .and(query_param("text", "Alexan"))
        .respond_with(ResponseTemplate::new(200).set_body_json(features()))
        .expect(1)
        .mount(&app.upstream)
        .await;
    let (status, _) = app
        .post_as(&token, "/v1/autocomplete", &json!({"text": "Alexan"}))
        .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn search_text_must_be_short_and_not_blank() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    for text in ["", "   ", &"x".repeat(201)] {
        let (status, _) = app
            .post_as(&token, "/v1/geocode", &json!({"text": text}))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{text:?}");
    }
}
