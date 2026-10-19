//! Every error a client can see, as `{"error": "<code>"}` with a fitting status.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    BadRequest,
    Unauthorized,
    IntegrityFailed,
    NotFound,
    MethodNotAllowed,
    OfferLocked,
    ReportClosed,
    NotAShortLink,
    NoDestination,
    NoRoute,
    TooManyRequests,
    IntegrityUnavailable,
    RouteServiceBusy,
    /// Logged with its message; the client sees only `internal`.
    Internal(String),
}

impl ApiError {
    /// An unexpected failure. The message goes to the log, so it must never contain request
    /// data (coordinates, search text, links, tokens).
    pub fn internal(message: impl Into<String>) -> ApiError {
        ApiError::Internal(message.into())
    }

    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            ApiError::BadRequest => (StatusCode::BAD_REQUEST, "bad_request"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::IntegrityFailed => (StatusCode::FORBIDDEN, "integrity_failed"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "method_not_allowed"),
            ApiError::OfferLocked => (StatusCode::CONFLICT, "offer_locked"),
            ApiError::ReportClosed => (StatusCode::CONFLICT, "report_closed"),
            ApiError::NotAShortLink => (StatusCode::UNPROCESSABLE_ENTITY, "not_a_short_link"),
            ApiError::NoDestination => (StatusCode::UNPROCESSABLE_ENTITY, "no_destination"),
            ApiError::NoRoute => (StatusCode::UNPROCESSABLE_ENTITY, "no_route"),
            ApiError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "too_many_requests"),
            ApiError::IntegrityUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "integrity_unavailable")
            }
            ApiError::RouteServiceBusy => (StatusCode::SERVICE_UNAVAILABLE, "route_service_busy"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let ApiError::Internal(message) = &self {
            tracing::error!(%message, "request failed");
        }
        let (status, code) = self.status_and_code();
        (status, Json(json!({ "error": code }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> ApiError {
        ApiError::internal(format!("database: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;

    async fn render(error: ApiError) -> (StatusCode, String) {
        let response = error.into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn errors_are_json_codes() {
        assert_eq!(
            render(ApiError::RouteServiceBusy).await,
            (
                StatusCode::SERVICE_UNAVAILABLE,
                r#"{"error":"route_service_busy"}"#.into()
            )
        );
        assert_eq!(
            render(ApiError::OfferLocked).await,
            (StatusCode::CONFLICT, r#"{"error":"offer_locked"}"#.into())
        );
    }

    #[tokio::test]
    async fn internal_errors_reveal_nothing() {
        let (status, body) = render(ApiError::internal("password=hunter2 at 52.52,13.40")).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, r#"{"error":"internal"}"#);
    }
}
