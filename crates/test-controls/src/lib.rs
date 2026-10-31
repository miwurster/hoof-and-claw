//! The controllable `Clock` and `PriceSource` and the HTTP endpoints that drive them.
//!
//! `server` compiles this crate in only with its Cargo feature `test-controls`, so the release build ships no way
//! to set the price or the time.

use axum::Router;

/// The routes of the test controls, which `web` serves under `/test-controls/`.
pub fn routes() -> Router {
    Router::new()
}
