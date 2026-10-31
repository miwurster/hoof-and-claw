//! `POST /api/guesses`: a Player places a Guess with a Direction and nothing else.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use game::{Clock, Fanout, PlaceError, Store};
use guessing::Refusal;
use identity::Token;
use serde::Deserialize;
use ts_rs::TS;

use crate::ErrorCode;
use crate::event::{Direction, OpenGuess};
use crate::stream::Api;

/// The body of the command to place a Guess; any other field, such as a price or a time, is ignored.
#[derive(Clone, Copy, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PlaceGuess {
    pub direction: Direction,
}

/// Answers `201` with the Open guess once it is saved and published (ADR 0014).
pub(crate) async fn place<C: Clock, F: Fanout, S: Store>(
    State(api): State<Api<C, F, S>>,
    Extension(token): Extension<Token>,
    command: Result<Json<PlaceGuess>, JsonRejection>,
) -> Result<(StatusCode, Json<OpenGuess>), ErrorCode> {
    let Json(command) = command.map_err(|rejection| match rejection {
        // Valid JSON without a Direction of `up` or `down`.
        JsonRejection::JsonDataError(_) => ErrorCode::InvalidDirection,
        _ => ErrorCode::BadRequest,
    })?;
    match api.game.place_guess(&token, command.direction.into()).await {
        Ok(open) => Ok((StatusCode::CREATED, Json(open.into()))),
        Err(PlaceError::Refused(Refusal::PriceDelayed)) => Err(ErrorCode::PriceDelayed),
        Err(PlaceError::Refused(Refusal::OpenGuessExists)) => Err(ErrorCode::OpenGuessExists),
        Err(PlaceError::Unavailable(error)) => {
            tracing::error!("cannot place a Guess: {error}");
            Err(ErrorCode::Internal)
        }
    }
}
