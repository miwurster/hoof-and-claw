//! The app's static files, with every other path falling back to `index.html`.

use std::path::Path;

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderValue, header};
use axum::middleware::{self, Next};
use axum::response::Response;
use tower_http::services::{ServeDir, ServeFile};

/// File names under `assets/` carry a content hash, so a browser may keep them forever.
const HASHED: HeaderValue = HeaderValue::from_static("public, max-age=31536000, immutable");
/// `index.html` and unhashed files are checked on every load, so a deploy reaches the next page load.
const UNHASHED: HeaderValue = HeaderValue::from_static("no-cache");

pub(crate) fn files(dir: &Path) -> Router {
    let files = ServeDir::new(dir).fallback(ServeFile::new(dir.join("index.html")));
    Router::new()
        .fallback_service(files)
        .layer(middleware::from_fn(cache_control))
}

async fn cache_control(request: Request, next: Next) -> Response {
    let hashed = request.uri().path().starts_with("/assets/");
    let mut response = next.run(request).await;
    if response.status().is_success() {
        let value = if hashed { HASHED } else { UNHASHED };
        response.headers_mut().insert(header::CACHE_CONTROL, value);
    }
    response
}
