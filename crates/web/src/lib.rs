//! The game's HTTP interface: the API under `/api/`, the health check, and the app's static files.
//!
//! The wire types here are the API contract; `ts-rs` exports them into `contract/` when this crate's tests run.

mod app;
mod cookie;
mod error;
mod event;
mod health;
mod stream;

use std::path::PathBuf;
use std::time::Duration;

use axum::middleware;
use axum::routing::get;
use axum::{Json, Router};
use game::{Clock, Fanout, Game};

pub use error::{ErrorBody, ErrorCode};
pub use event::{
    Direction, OpenGuess, Outcome, PriceChanged, ResolvedGuess, ServerEvent, Snapshot,
};
pub use health::Health;

use crate::stream::Api;

/// Everything the router serves.
pub struct Config<C, F> {
    /// The running game.
    pub game: Game<C, F>,
    /// The commit the server was built from, reported by `/health` and every snapshot.
    pub build: String,
    /// The directory holding the built app, with `index.html` at its root.
    pub app_dir: PathBuf,
    /// How often an open stream gets a keepalive comment.
    pub keepalive: Duration,
    /// The test controls' routes, served under `/test-controls/` in a build with the test controls.
    pub test_controls: Option<Router>,
}

/// Builds the router of the whole HTTP interface.
pub fn router<C: Clock, F: Fanout>(config: Config<C, F>) -> Router {
    let health = Health::new(config.build.clone(), config.test_controls.is_some());
    let api = Router::new()
        .route("/snapshot", get(stream::snapshot::<C, F>))
        .route("/events", get(stream::events::<C, F>))
        .fallback(error::not_found)
        .with_state(Api {
            game: config.game,
            build: config.build,
            keepalive: config.keepalive,
        })
        .layer(middleware::from_fn(error::as_json))
        .layer(middleware::from_fn(cookie::on_every_response));
    let test_controls = config
        .test_controls
        .unwrap_or_default()
        .fallback(error::not_found)
        .layer(middleware::from_fn(error::as_json));
    Router::new()
        .route(
            "/health",
            get(move || std::future::ready(Json(health.clone()))),
        )
        .nest("/api", api)
        .nest("/test-controls", test_controls)
        .fallback_service(
            app::files(&config.app_dir).layer(middleware::from_fn(cookie::on_the_page)),
        )
}
