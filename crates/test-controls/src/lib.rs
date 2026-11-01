//! The controllable `Clock`, `PriceSource` and store, and the HTTP endpoints that drive them.
//!
//! `server` compiles this crate in only with its Cargo feature `test-controls`, so the release build ships no way
//! to set the price or the time, or to make the store fail.

mod clock;
mod command;
mod price_source;
mod store;

use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use game::{Fanout, Game, Store};
use prices::Price;

pub use clock::{BeyondTheLastTime, ControllableClock};
pub use command::{AdvanceClock, SendPrice, SetClock, SetConnection, SetStore, StoreFailure};
pub use price_source::{ControllableSource, SourceControls, controllable_source};
pub use store::{ControllableStore, Failed, Failing, StoreControls, controllable_store};

/// What the endpoints drive: the controllable clock, source and store, and the game they feed.
#[derive(Clone)]
pub struct Controls<F, S> {
    pub clock: ControllableClock,
    pub source: SourceControls,
    pub store: StoreControls,
    pub game: Game<ControllableClock, F, S>,
}

/// The routes of the test controls, which `web` serves under `/test-controls/`.
pub fn routes<F: Fanout, S: Store>(controls: Controls<F, S>) -> Router {
    Router::new()
        .route("/price", post(send_price::<F, S>))
        .route("/connection", post(set_connection::<F, S>))
        .route("/clock", post(set_clock::<F, S>))
        .route("/clock/advance", post(advance_clock::<F, S>))
        .route("/store", post(set_store::<F, S>))
        .route("/reset", post(reset::<F, S>))
        .with_state(controls)
}

async fn send_price<F: Fanout, S: Store>(
    State(controls): State<Controls<F, S>>,
    Json(command): Json<SendPrice>,
) -> StatusCode {
    controls.source.send(Price::from_cents(command.price));
    StatusCode::NO_CONTENT
}

async fn set_connection<F: Fanout, S: Store>(
    State(controls): State<Controls<F, S>>,
    Json(command): Json<SetConnection>,
) -> StatusCode {
    controls.source.set_connected(command.connected);
    StatusCode::NO_CONTENT
}

async fn set_clock<F: Fanout, S: Store>(
    State(controls): State<Controls<F, S>>,
    Json(command): Json<SetClock>,
) -> StatusCode {
    controls.clock.set(command.time);
    StatusCode::NO_CONTENT
}

async fn advance_clock<F: Fanout, S: Store>(
    State(controls): State<Controls<F, S>>,
    Json(command): Json<AdvanceClock>,
) -> StatusCode {
    match controls
        .clock
        .advance(Duration::from_millis(command.milliseconds))
    {
        Ok(()) => StatusCode::NO_CONTENT,
        Err(BeyondTheLastTime) => StatusCode::UNPROCESSABLE_ENTITY,
    }
}

async fn set_store<F: Fanout, S: Store>(
    State(controls): State<Controls<F, S>>,
    Json(command): Json<SetStore>,
) -> StatusCode {
    controls.store.set(match command.fails {
        StoreFailure::Nothing => Failing::Nothing,
        StoreFailure::Resolutions => Failing::Resolutions,
        StoreFailure::Everything => Failing::Everything,
    });
    StatusCode::NO_CONTENT
}

/// Puts the Price source back into its start state, Price delayed with no price, and makes the store fail nothing,
/// before each acceptance test.
async fn reset<F: Fanout, S: Store>(State(controls): State<Controls<F, S>>) -> StatusCode {
    controls.store.set(Failing::Nothing);
    controls.source.set_connected(false);
    controls.game.forget_the_price();
    StatusCode::NO_CONTENT
}
