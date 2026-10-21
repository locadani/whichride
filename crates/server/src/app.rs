//! The HTTP router and its middleware.

use axum::extract::{DefaultBodyLimit, MatchedPath, Request};
use axum::routing::{get, post};
use std::any::Any;

use axum::response::{IntoResponse, Response};
use axum::{Router, middleware};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::error::ApiError;
use crate::state::AppState;

/// Largest request body accepted; real requests are well under 2 KiB.
pub const BODY_LIMIT_BYTES: usize = 16 * 1024;

/// The API, and the website behind it when `site` is given (`site::router`).
pub fn router(state: AppState, site: Option<Router>) -> Router {
    let api = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/catalogue", get(crate::catalogue::get_catalogue))
        .route("/v1/installs", post(crate::installs::register))
        .route("/v1/reports", post(crate::crowd::post_report))
        .route("/v1/reports/{id}/votes", post(crate::crowd::post_vote))
        .route(
            "/v1/offers/{id}/confirmations",
            post(crate::crowd::post_confirmation),
        )
        .route("/v1/route", post(crate::routing::post_route))
        .route("/v1/geocode", post(crate::routing::post_geocode))
        .route("/v1/autocomplete", post(crate::routing::post_autocomplete))
        .route("/v1/expand", post(crate::expand::post_expand))
        .method_not_allowed_fallback(|| async { ApiError::MethodNotAllowed });
    let api = match site {
        Some(site) => api.fallback_service(site),
        None => api.fallback(|| async { ApiError::NotFound }),
    };
    api.with_state(state)
        .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
        .layer(CatchPanicLayer::custom(panic_response))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(request_span)
                .on_request(())
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(middleware::map_request(drop_client_request_id))
}

/// A handler that panicked answers like any other failure; the panic message is not logged, as
/// it could hold request data.
fn panic_response(_: Box<dyn Any + Send + 'static>) -> Response {
    tracing::error!("request handler panicked");
    ApiError::internal("panic").into_response()
}

/// A client's own `x-request-id` would be logged and echoed back; it could carry anything
/// (a location, say), so every request gets a fresh ID instead.
async fn drop_client_request_id(mut request: Request) -> Request {
    request.headers_mut().remove("x-request-id");
    request
}

/// The log span of a request: method, the *route template* (`/v1/reports/{id}/votes`, never the
/// actual path, so nothing a client puts in a URL reaches the logs) and the request ID. Headers
/// and bodies are never logged (spec §9).
fn request_span(request: &Request) -> tracing::Span {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str);
    let id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("-");
    tracing::info_span!("request", method = %request.method(), route, id)
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use axum::http::StatusCode;

    use super::*;

    #[tokio::test]
    async fn a_panic_is_a_json_internal_error() {
        let response = panic_response(Box::new("at 52.52,13.41"));
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(&body[..], br#"{"error":"internal"}"#);
    }
}
