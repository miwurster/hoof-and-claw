//! The `guessing` store port on the tables `players` and `guesses`.

use guessing::{
    Direction, GuessId, OpenGuess, Outcome, PlayerId, Resolution, ResolvedGuess, Score, Standing,
    Store, StoreError,
};
use jiff::Timestamp;
use prices::{Price, ReceivedPrice, Source, TradeId};

use crate::Database;

fn unavailable(error: impl std::error::Error + Send + Sync + 'static) -> StoreError {
    StoreError::Unavailable(Box::new(error))
}

#[derive(Debug, thiserror::Error)]
#[error("the database holds {0}, which the game cannot read")]
struct Corrupt(String);

fn corrupt(what: impl Into<String>) -> StoreError {
    unavailable(Corrupt(what.into()))
}

pub(crate) fn nanoseconds(time: Timestamp) -> Result<i64, StoreError> {
    i64::try_from(time.as_nanosecond()).map_err(|_| corrupt(format!("a time {time} beyond 2262")))
}

pub(crate) fn timestamp(nanoseconds: i64) -> Result<Timestamp, StoreError> {
    Timestamp::from_nanosecond(i128::from(nanoseconds))
        .map_err(|_| corrupt(format!("the time {nanoseconds}")))
}

fn cents(price: Price) -> Result<i64, StoreError> {
    i64::try_from(price.cents()).map_err(|_| corrupt(format!("the price {price:?}")))
}

fn price(cents: i64) -> Result<Price, StoreError> {
    u64::try_from(cents)
        .map(Price::from_cents)
        .map_err(|_| corrupt(format!("the price {cents}")))
}

const fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Up => "up",
        Direction::Down => "down",
    }
}

fn direction(name: &str) -> Result<Direction, StoreError> {
    match name {
        "up" => Ok(Direction::Up),
        "down" => Ok(Direction::Down),
        _ => Err(corrupt(format!("the direction {name:?}"))),
    }
}

fn trade_number(trade_id: Option<TradeId>) -> Result<Option<i64>, StoreError> {
    trade_id
        .map(|id| i64::try_from(id.value()).map_err(|_| corrupt(format!("the trade ID {id:?}"))))
        .transpose()
}

fn trade_id(number: Option<i64>) -> Result<Option<TradeId>, StoreError> {
    number
        .map(|number| {
            u64::try_from(number)
                .map(TradeId::new)
                .map_err(|_| corrupt(format!("the trade ID {number}")))
        })
        .transpose()
}

const fn outcome_name(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Correct => "correct",
        Outcome::Wrong => "wrong",
    }
}

fn outcome(name: &str) -> Result<Outcome, StoreError> {
    match name {
        "correct" => Ok(Outcome::Correct),
        "wrong" => Ok(Outcome::Wrong),
        _ => Err(corrupt(format!("the Outcome {name:?}"))),
    }
}

fn guess_id(text: &str) -> Result<GuessId, StoreError> {
    text.parse::<GuessId>()
        .map_err(|_| corrupt(format!("the guess ID {text:?}")))
}

fn received(
    cents: i64,
    source_name: &str,
    trade: Option<i64>,
    received_at: i64,
) -> Result<ReceivedPrice, StoreError> {
    Ok(ReceivedPrice {
        price: price(cents)?,
        source: source(source_name)?,
        trade_id: trade_id(trade)?,
        received_at: timestamp(received_at)?,
    })
}

const fn source_name(source: Source) -> &'static str {
    match source {
        Source::Bitstamp => "bitstamp",
    }
}

fn source(name: &str) -> Result<Source, StoreError> {
    match name {
        "bitstamp" => Ok(Source::Bitstamp),
        _ => Err(corrupt(format!("the Price source {name:?}"))),
    }
}

impl Store for Database {
    async fn place(&self, guess: &OpenGuess) -> Result<(), StoreError> {
        let id = guess.id.to_string();
        let player = guess.player.to_string();
        let direction = direction_name(guess.direction);
        let placed_at = nanoseconds(guess.placed_at)?;
        let start_price = cents(guess.start.price)?;
        let start_received_at = nanoseconds(guess.start.received_at)?;
        let start_source = source_name(guess.start.source);
        let start_trade_id = trade_number(guess.start.trade_id)?;

        let mut transaction = self.pool.begin().await.map_err(unavailable)?;
        sqlx::query!(
            "INSERT INTO players (id, score, created_at) VALUES (?, 0, ?) ON CONFLICT (id) DO NOTHING",
            player,
            placed_at,
        )
        .execute(&mut *transaction)
        .await
        .map_err(unavailable)?;
        let inserted = sqlx::query!(
            "INSERT INTO guesses
                (id, player_id, direction, placed_at, start_price_cents, start_received_at, start_source, start_trade_id,
                 state)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'open')",
            id,
            player,
            direction,
            placed_at,
            start_price,
            start_received_at,
            start_source,
            start_trade_id,
        )
        .execute(&mut *transaction)
        .await;
        match inserted {
            Ok(_) => transaction.commit().await.map_err(unavailable),
            // The partial unique index allows one Open guess per Player.
            Err(sqlx::Error::Database(error)) if error.is_unique_violation() => {
                Err(StoreError::OpenGuessExists)
            }
            Err(error) => Err(unavailable(error)),
        }
    }

    async fn resolve(&self, resolved: &[ResolvedGuess]) -> Result<Vec<Resolution>, StoreError> {
        let mut transaction = self.pool.begin().await.map_err(unavailable)?;
        let mut saved = Vec::with_capacity(resolved.len());
        for resolution in resolved {
            let id = resolution.guess.id.to_string();
            let player = resolution.guess.player.to_string();
            let end_price = cents(resolution.end.price)?;
            let end_received_at = nanoseconds(resolution.end.received_at)?;
            let end_source = source_name(resolution.end.source);
            let end_trade_id = trade_number(resolution.end.trade_id)?;
            let outcome = outcome_name(resolution.outcome);
            let change = resolution.outcome.score_change();
            // Only a Guess that is still open changes, so a retried save cannot resolve it twice.
            let changed = sqlx::query!(
                "UPDATE guesses
                 SET state = 'resolved', end_price_cents = ?, end_received_at = ?, end_source = ?, end_trade_id = ?,
                     outcome = ?, resolved_at = ?
                 WHERE id = ? AND state = 'open'",
                end_price,
                end_received_at,
                end_source,
                end_trade_id,
                outcome,
                end_received_at,
                id,
            )
            .execute(&mut *transaction)
            .await
            .map_err(unavailable)?
            .rows_affected();
            if changed == 0 {
                continue;
            }
            let score = sqlx::query_scalar!(
                "UPDATE players SET score = score + ? WHERE id = ? RETURNING score",
                change,
                player,
            )
            .fetch_one(&mut *transaction)
            .await
            .map_err(unavailable)?;
            saved.push(Resolution {
                guess: *resolution,
                score: Score::new(score),
            });
        }
        transaction.commit().await.map_err(unavailable)?;
        Ok(saved)
    }

    async fn standing(&self, player: PlayerId) -> Result<Standing, StoreError> {
        let id = player.to_string();
        let mut connection = self.pool.acquire().await.map_err(unavailable)?;
        let score = sqlx::query_scalar!("SELECT score FROM players WHERE id = ?", id)
            .fetch_optional(&mut *connection)
            .await
            .map_err(unavailable)?
            .unwrap_or(0);
        let open = sqlx::query!(
            "SELECT id, direction, placed_at, start_price_cents, start_received_at, start_source, start_trade_id
             FROM guesses WHERE player_id = ? AND state = 'open'",
            id,
        )
        .fetch_optional(&mut *connection)
        .await
        .map_err(unavailable)?;
        let open_guess = open
            .map(|row| {
                Ok::<_, StoreError>(OpenGuess {
                    id: guess_id(&row.id)?,
                    player,
                    direction: direction(&row.direction)?,
                    start: received(
                        row.start_price_cents,
                        &row.start_source,
                        row.start_trade_id,
                        row.start_received_at,
                    )?,
                    placed_at: timestamp(row.placed_at)?,
                })
            })
            .transpose()?;
        let last = sqlx::query!(
            r#"SELECT id, direction, placed_at, start_price_cents, start_received_at, start_source, start_trade_id,
                 end_price_cents AS "end_price_cents!", end_received_at AS "end_received_at!",
                 end_source AS "end_source!", end_trade_id, outcome AS "outcome!"
             FROM guesses WHERE player_id = ? AND state = 'resolved'
             ORDER BY resolved_at DESC LIMIT 1"#,
            id,
        )
        .fetch_optional(&mut *connection)
        .await
        .map_err(unavailable)?;
        let last_resolved = last
            .map(|row| {
                Ok::<_, StoreError>(ResolvedGuess {
                    guess: OpenGuess {
                        id: guess_id(&row.id)?,
                        player,
                        direction: direction(&row.direction)?,
                        start: received(
                            row.start_price_cents,
                            &row.start_source,
                            row.start_trade_id,
                            row.start_received_at,
                        )?,
                        placed_at: timestamp(row.placed_at)?,
                    },
                    end: received(
                        row.end_price_cents,
                        &row.end_source,
                        row.end_trade_id,
                        row.end_received_at,
                    )?,
                    outcome: outcome(&row.outcome)?,
                })
            })
            .transpose()?;
        Ok(Standing {
            score: Score::new(score),
            open_guess,
            last_resolved,
        })
    }
}
