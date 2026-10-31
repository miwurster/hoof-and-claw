//! The `guessing` store port on the tables `players` and `guesses`.

use guessing::{Direction, GuessId, OpenGuess, PlayerId, Score, Standing, Store, StoreError};
use jiff::Timestamp;
use prices::{Price, ReceivedPrice, Source};

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
                (id, player_id, direction, placed_at, start_price_cents, start_received_at, start_source, state)
             VALUES (?, ?, ?, ?, ?, ?, ?, 'open')",
            id,
            player,
            direction,
            placed_at,
            start_price,
            start_received_at,
            start_source,
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

    async fn standing(&self, player: PlayerId) -> Result<Standing, StoreError> {
        let id = player.to_string();
        let mut connection = self.pool.acquire().await.map_err(unavailable)?;
        let score = sqlx::query_scalar!("SELECT score FROM players WHERE id = ?", id)
            .fetch_optional(&mut *connection)
            .await
            .map_err(unavailable)?
            .unwrap_or(0);
        let open = sqlx::query!(
            "SELECT id, direction, placed_at, start_price_cents, start_received_at, start_source
             FROM guesses WHERE player_id = ? AND state = 'open'",
            id,
        )
        .fetch_optional(&mut *connection)
        .await
        .map_err(unavailable)?;
        let open_guess = open
            .map(|row| {
                Ok::<_, StoreError>(OpenGuess {
                    id: row
                        .id
                        .parse::<GuessId>()
                        .map_err(|_| corrupt(format!("the guess ID {:?}", row.id)))?,
                    player,
                    direction: direction(&row.direction)?,
                    start: ReceivedPrice {
                        price: price(row.start_price_cents)?,
                        source: source(&row.start_source)?,
                        received_at: timestamp(row.start_received_at)?,
                    },
                    placed_at: timestamp(row.placed_at)?,
                })
            })
            .transpose()?;
        Ok(Standing {
            score: Score::new(score),
            open_guess,
        })
    }
}
