mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const PLACE: &str = "https://www.google.com/maps/place/Alexanderplatz,+10178+Berlin/@52.52,13.41,15z/data=!3m1!4b1!4m6!3m5!1s0x47a84e1f1d4d3f0b:0x3b2d0f4c1c1e2a0!8m2!3d52.5219184!4d13.4132147!16zL20vMDFsNnQ1?entry=ttu";

fn alexanderplatz() -> serde_json::Value {
    json!({"type": "coordinates", "at": {"lat_e7": 525_219_184, "lon_e7": 134_132_147},
           "label": "Alexanderplatz, 10178 Berlin"})
}

async fn redirect(app: &TestApp, from: &str, to: &str) {
    Mock::given(method("GET"))
        .and(path(from))
        .respond_with(ResponseTemplate::new(302).insert_header("location", to))
        .mount(&app.upstream)
        .await;
}

#[tokio::test]
async fn a_short_link_expands_to_its_destination() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    redirect(&app, "/AbC123", PLACE).await;
    let shared = "Alexanderplatz\nhttps://maps.app.goo.gl/AbC123?g_st=ic";
    let (status, body) = app
        .post_as(&token, "/v1/expand", &json!({"url": shared}))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, alexanderplatz());
}

#[tokio::test]
async fn the_eu_consent_page_is_skipped() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    let consent = format!(
        "https://consent.google.com/ml?continue={}&gl=DE&hl=de&pc=m&src=1",
        url::form_urlencoded::byte_serialize(PLACE.as_bytes()).collect::<String>()
    );
    redirect(&app, "/AbC123", &consent).await;
    let (status, body) = app
        .post_as(
            &token,
            "/v1/expand",
            &json!({"url": "https://maps.app.goo.gl/AbC123"}),
        )
        .await;
    assert_eq!((status, body), (StatusCode::OK, alexanderplatz()));
}

#[tokio::test]
async fn a_chain_of_short_links_is_followed_but_not_forever() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    redirect(&app, "/maps/old", "https://maps.app.goo.gl/AbC123").await;
    redirect(&app, "/AbC123", PLACE).await;
    let (status, _) = app
        .post_as(
            &token,
            "/v1/expand",
            &json!({"url": "https://goo.gl/maps/old"}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    redirect(&app, "/Loop", "https://maps.app.goo.gl/Loop").await;
    let (status, body) = app
        .post_as(
            &token,
            "/v1/expand",
            &json!({"url": "https://maps.app.goo.gl/Loop"}),
        )
        .await;
    assert_eq!(
        (status, body),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            json!({"error": "route_service_busy"})
        )
    );
}

#[tokio::test]
async fn only_short_links_are_fetched() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(302))
        .expect(0)
        .mount(&app.upstream)
        .await;
    for url in [
        "https://example.com/AbC123",
        "http://169.254.169.254/computeMetadata/v1/",
        "https://www.google.com/maps/place/X",
        "no link at all",
    ] {
        let (status, body) = app
            .post_as(&token, "/v1/expand", &json!({"url": url}))
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{url}");
        assert_eq!(body, json!({"error": "not_a_short_link"}), "{url}");
    }
}

#[tokio::test]
async fn a_redirect_away_from_google_maps_has_no_destination() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    redirect(&app, "/Elsewhere", "https://example.com/phishing").await;
    Mock::given(method("GET"))
        .and(path("/Gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&app.upstream)
        .await;
    for id in ["Elsewhere", "Gone"] {
        let url = format!("https://maps.app.goo.gl/{id}");
        let (status, body) = app
            .post_as(&token, "/v1/expand", &json!({"url": url}))
            .await;
        assert_eq!(
            (status, body),
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({"error": "no_destination"})
            ),
            "{id}"
        );
    }
}

#[tokio::test]
async fn expansion_needs_a_registered_install() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let (status, _) = app
        .post(
            "/v1/expand",
            &json!({"url": "https://maps.app.goo.gl/AbC123"}),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn google_rate_limiting_us_is_route_service_busy() {
    let Some(app) = TestApp::start().await else {
        return;
    };
    let token = app.register().await;
    for (id, status) in [("Limited", 429), ("Down", 503)] {
        Mock::given(method("GET"))
            .and(path(format!("/{id}")))
            .respond_with(ResponseTemplate::new(status))
            .mount(&app.upstream)
            .await;
        let url = format!("https://maps.app.goo.gl/{id}");
        let (status, body) = app
            .post_as(&token, "/v1/expand", &json!({"url": url}))
            .await;
        assert_eq!(
            (status, body),
            (
                StatusCode::SERVICE_UNAVAILABLE,
                json!({"error": "route_service_busy"})
            ),
            "{id}"
        );
    }
}
