//! The game's HTTP interface: the API under `/api/`, the health check, and the app's static files.
//!
//! The wire types here are the API contract; `ts-rs` exports them into `contract/` when this crate's tests run.

mod app;
mod error;
mod health;

use std::path::PathBuf;

use axum::routing::get;
use axum::{Json, Router};

pub use error::{ErrorBody, ErrorCode};
pub use health::Health;

/// What the router serves besides the game itself.
pub struct Config {
    /// The commit the server was built from, reported by `/health`.
    pub build: String,
    /// The directory holding the built app, with `index.html` at its root.
    pub app_dir: PathBuf,
    /// The test controls' routes, served under `/test-controls/` in a build with the test controls.
    pub test_controls: Option<Router>,
}

/// Builds the router of the whole HTTP interface.
pub fn router(config: Config) -> Router {
    let health = Health::new(config.build, config.test_controls.is_some());
    let test_controls = config.test_controls.unwrap_or_default();
    Router::new()
        .route(
            "/health",
            get(move || std::future::ready(Json(health.clone()))),
        )
        .nest("/api", Router::new().fallback(error::not_found))
        .nest("/test-controls", test_controls.fallback(error::not_found))
        .fallback_service(app::files(&config.app_dir))
}
