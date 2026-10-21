mod common;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use common::{Options, TestApp, fake_site};
use tower::ServiceExt;
use whichride_server::site;

async fn get(
    router: &Router,
    path: &str,
    headers: &[(&str, &str)],
) -> (StatusCode, HeaderMap, String) {
    let mut request = Request::get(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    (status, headers, String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
async fn the_root_redirects_to_the_visitors_language() {
    let router = site::router(&fake_site());
    for (accept, expected) in [
        (Some("it-IT,it;q=0.9,en;q=0.8"), "/it/"),
        (Some("fr-FR, de;q=0.5, tr;q=0.9"), "/tr/"),
        (Some("fr"), "/en/"),
        (None, "/en/"),
    ] {
        let headers: Vec<(&str, &str)> =
            accept.map(|a| ("accept-language", a)).into_iter().collect();
        let (status, headers, _) = get(&router, "/", &headers).await;
        assert_eq!(status, StatusCode::FOUND, "{accept:?}");
        assert_eq!(headers[header::LOCATION], expected, "{accept:?}");
        assert_eq!(headers[header::VARY], "Accept-Language");
    }
}

#[tokio::test]
async fn a_language_page_is_served_revalidated_and_hardened() {
    let (status, headers, body) = get(&site::router(&fake_site()), "/it/", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "<h1>it</h1>");
    assert!(
        headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert_eq!(headers[header::CACHE_CONTROL], "no-cache");
    assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert_eq!(headers[header::REFERRER_POLICY], "no-referrer");
    assert_eq!(headers[header::X_FRAME_OPTIONS], "DENY");
}

#[tokio::test]
async fn a_language_without_its_slash_is_redirected() {
    let (status, headers, _) = get(&site::router(&fake_site()), "/en", &[]).await;
    assert!(status.is_redirection(), "{status}");
    assert_eq!(headers[header::LOCATION], "/en/");
}

#[tokio::test]
async fn hashed_assets_are_immutable_and_precompressed() {
    let (status, headers, body) = get(
        &site::router(&fake_site()),
        "/_astro/app.1a2b3c.js",
        &[("accept-encoding", "gzip")],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_ENCODING], "gzip");
    assert_eq!(body, "gzipped");
    assert_eq!(
        headers[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
}

#[tokio::test]
async fn unknown_paths_get_the_404_page_never_cached_as_immutable() {
    let router = site::router(&fake_site());
    for path in ["/nope/", "/_astro/gone.js"] {
        let (status, headers, body) = get(&router, path, &[]).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(body, "<h1>missing</h1>", "{path}");
        assert_eq!(headers[header::CACHE_CONTROL], "no-cache", "{path}");
    }
}

#[tokio::test]
async fn unknown_api_paths_keep_the_json_404() {
    let (status, headers, body) = get(&site::router(&fake_site()), "/v1/nope", &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );
    assert_eq!(body, r#"{"error":"not_found"}"#);
}

#[test]
fn a_directory_without_a_built_site_is_refused() {
    let empty = std::env::temp_dir().join(format!("whichride-empty-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&empty).unwrap();
    assert!(site::check(&empty).unwrap_err().contains("en/index.html"));
    assert!(site::check(&fake_site()).is_ok());
}

#[tokio::test]
async fn the_api_and_the_site_share_one_router() {
    let Some(app) = TestApp::start_with(Options {
        site: Some(fake_site()),
        ..Options::default()
    })
    .await
    else {
        return;
    };
    let (status, body) = app.get("/v1/catalogue").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _, page) = get(&app.router, "/en/", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page, "<h1>en</h1>");
    let (status, _, error) = get(&app.router, "/v1/nope", &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error, r#"{"error":"not_found"}"#);
}
