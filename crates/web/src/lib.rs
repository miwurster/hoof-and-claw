//! The game's HTTP interface: the API under `/api/`, the health check, and the app's static files.
//!
//! The wire types here are the API contract; `ts-rs` exports them into `contract/` when this crate's tests run.

mod app;
mod client_ip;
mod cookie;
mod error;
mod event;
mod guess;
mod health;
mod stream;
mod streams_per_ip;

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::time::Duration;

use axum::response::IntoResponse as _;
use axum::routing::{get, post};
use axum::{Json, Router, middleware};
use game::{Clock, Fanout, Game, Store};
use tower_governor::GovernorError;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;

pub use client_ip::ClientIp;
pub use error::{ErrorBody, ErrorCode};
pub use event::{
    Direction, GuessPlaced, GuessResolved, OpenGuess, Outcome, PriceChanged, ResolvedGuess,
    ServerEvent, Snapshot,
};
pub use guess::PlaceGuess;
pub use health::Health;

use crate::cookie::Cookies;
use crate::stream::Api;
use crate::streams_per_ip::StreamsPerIp;

/// Everything the router serves.
pub struct Config<C, F, S> {
    /// The running game.
    pub game: Game<C, F, S>,
    /// The commit the server was built from, reported by `/health` and every snapshot.
    pub build: String,
    /// The directory holding the built app, with `index.html` at its root.
    pub app_dir: PathBuf,
    /// How often an open stream gets the keepalive event.
    pub keepalive: Duration,
    /// Which address a request comes from, for the limits.
    pub client_ip: ClientIp,
    pub limits: Limits,
    /// The test controls' routes, served under `/test-controls/` in a build with the test controls.
    pub test_controls: Option<Router>,
}

/// The per-IP limits (spec #26, "Rate limits").
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub guesses_per_minute: NonZeroU32,
    pub guess_burst: NonZeroU32,
    pub cookies_per_hour: NonZeroU32,
    pub cookie_burst: NonZeroU32,
    /// How many streams one IP may hold open at once; the next is refused with `rateLimited`.
    pub streams_per_ip: usize,
}

/// Places a Guess at most `guesses_per_minute` times a minute per IP, after a burst of `guess_burst`.
#[expect(
    clippy::expect_used,
    reason = "the builder refuses only a zero period or burst, which the non-zero limits cannot hold"
)]
fn guess_limit(
    client_ip: ClientIp,
    limits: Limits,
) -> GovernorLayer<
    ClientIp,
    governor::middleware::NoOpMiddleware<governor::clock::QuantaInstant>,
    axum::body::Body,
> {
    let config = GovernorConfigBuilder::default()
        .key_extractor(client_ip)
        .period(Duration::from_secs(60) / limits.guesses_per_minute.get())
        .burst_size(limits.guess_burst.get())
        .finish()
        .expect("a non-zero rate and burst make a valid limit");
    GovernorLayer::new(config).error_handler(|error| match error {
        GovernorError::TooManyRequests { .. } => ErrorCode::RateLimited.into_response(),
        GovernorError::UnableToExtractKey | GovernorError::Other { .. } => {
            ErrorCode::Internal.into_response()
        }
    })
}

/// Builds the router of the whole HTTP interface.
pub fn router<C: Clock, F: Fanout, S: Store>(config: Config<C, F, S>) -> Router {
    let health = Health::new(config.build.clone(), config.test_controls.is_some());
    let cookies = Cookies::new(
        config.client_ip.clone(),
        config.limits.cookies_per_hour,
        config.limits.cookie_burst,
    );
    let place =
        post(guess::place::<C, F, S>).layer(guess_limit(config.client_ip.clone(), config.limits));
    let api = Router::new()
        .route("/snapshot", get(stream::snapshot::<C, F, S>))
        .route("/events", get(stream::events::<C, F, S>))
        .route("/guesses", place)
        .fallback(error::not_found)
        .with_state(Api {
            game: config.game,
            build: config.build,
            keepalive: config.keepalive,
            streams: StreamsPerIp::new(config.limits.streams_per_ip),
            client_ip: config.client_ip,
        })
        .layer(middleware::from_fn(error::as_json))
        .layer(middleware::from_fn_with_state(
            cookies.clone(),
            cookie::on_every_response,
        ));
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
            app::files(&config.app_dir)
                .layer(middleware::from_fn_with_state(cookies, cookie::on_the_page)),
        )
}
