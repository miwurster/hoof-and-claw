use axum::Json;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use ts_rs::TS;

/// The code of every error under `/api/` and `/test-controls/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ErrorCode {
    OpenGuessExists,
    PriceDelayed,
    InvalidDirection,
    BadRequest,
    NotFound,
    RateLimited,
    Internal,
}

/// The body of every error under `/api/` and `/test-controls/`: `{ "error": "<code>" }`.
#[derive(Clone, Copy, Debug, Serialize, TS)]
#[ts(export)]
pub struct ErrorBody {
    error: ErrorCode,
}

impl ErrorCode {
    fn status(self) -> StatusCode {
        match self {
            Self::OpenGuessExists | Self::PriceDelayed => StatusCode::CONFLICT,
            Self::InvalidDirection => StatusCode::UNPROCESSABLE_ENTITY,
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ErrorCode {
    fn into_response(self) -> Response {
        (self.status(), Json(ErrorBody { error: self })).into_response()
    }
}

pub(crate) async fn not_found() -> ErrorCode {
    ErrorCode::NotFound
}

/// Turns axum's plain-text answers, such as an extractor's rejection, into `{ "error": "<code>" }`.
pub(crate) async fn as_json(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    let status = response.status();
    let is_json = response
        .headers()
        .get(CONTENT_TYPE)
        .is_some_and(|value| value.as_bytes().starts_with(b"application/json"));
    if is_json || !(status.is_client_error() || status.is_server_error()) {
        return response;
    }
    let code = match status {
        StatusCode::BAD_REQUEST
        | StatusCode::UNSUPPORTED_MEDIA_TYPE
        | StatusCode::UNPROCESSABLE_ENTITY
        | StatusCode::PAYLOAD_TOO_LARGE => ErrorCode::BadRequest,
        // A known path asked with a method it does not answer is no route either.
        StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED => ErrorCode::NotFound,
        StatusCode::TOO_MANY_REQUESTS => ErrorCode::RateLimited,
        _ => ErrorCode::Internal,
    };
    let mut json = code.into_response();
    json.headers_mut().extend(
        response
            .headers()
            .iter()
            .filter(|(name, _)| {
                *name != CONTENT_TYPE && *name != axum::http::header::CONTENT_LENGTH
            })
            .map(|(name, value)| (name.clone(), value.clone())),
    );
    json
}
