use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use ts_rs::TS;

/// The code of every error under `/api/` and `/test-controls/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ErrorCode {
    NotFound,
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
            Self::NotFound => StatusCode::NOT_FOUND,
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
