use std::time::Duration;

use jiff::{SignedDuration, Timestamp};
use prices::ReceivedPrice;

use crate::{Direction, GuessId, PlayerId};

/// A Guess falls due this long after it was placed.
pub const GUESS_DURATION: Duration = Duration::from_secs(60);

/// A Guess that is not yet resolved; a Player has at most one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenGuess {
    pub id: GuessId,
    pub player: PlayerId,
    pub direction: Direction,
    /// The last price the server received before it accepted the Guess.
    pub start: ReceivedPrice,
    pub placed_at: Timestamp,
}

/// Why the game refuses to place a Guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("the price is delayed, or no price has been received yet")]
    PriceDelayed,
    #[error("the Player already has an Open guess")]
    OpenGuessExists,
}

impl OpenGuess {
    /// Places a Guess at `now` from the last price received, which needs a live price (ADR 0007).
    ///
    /// # Errors
    ///
    /// Refuses with `PriceDelayed` while the price is delayed or before any price was received.
    pub fn place(
        id: GuessId,
        player: PlayerId,
        direction: Direction,
        last_received: Option<ReceivedPrice>,
        delayed: bool,
        now: Timestamp,
    ) -> Result<Self, Refusal> {
        match last_received {
            Some(start) if !delayed => Ok(Self {
                id,
                player,
                direction,
                start,
                placed_at: now,
            }),
            _ => Err(Refusal::PriceDelayed),
        }
    }

    /// When the Guess falls due: 60 seconds after it was placed.
    #[must_use]
    pub fn due_at(&self) -> Timestamp {
        self.placed_at
            .saturating_add(SignedDuration::from_secs(60))
            .unwrap_or(Timestamp::MAX)
    }

    /// The time left at `now` until the Guess falls due, 0 once it has.
    #[must_use]
    pub fn remaining(&self, now: Timestamp) -> Duration {
        Duration::try_from(self.due_at().duration_since(now)).unwrap_or(Duration::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use prices::{Price, Source};

    use super::*;

    const NOW: Timestamp = Timestamp::constant(1_798_761_600, 0);

    fn received(cents: u64) -> ReceivedPrice {
        ReceivedPrice {
            price: Price::from_cents(cents),
            source: Source::Bitstamp,
            received_at: NOW,
        }
    }

    fn place(last: Option<ReceivedPrice>, delayed: bool) -> Result<OpenGuess, Refusal> {
        OpenGuess::place(
            GuessId::new(NOW, [1; 10]),
            PlayerId::new(NOW, [2; 10]),
            Direction::Up,
            last,
            delayed,
            NOW,
        )
    }

    #[test]
    fn a_guess_starts_from_the_last_price_received() {
        let guess = place(Some(received(6_400_050)), false).unwrap();

        assert_eq!(guess.start, received(6_400_050));
        assert_eq!(guess.direction, Direction::Up);
        assert_eq!(guess.placed_at, NOW);
    }

    #[test]
    fn while_the_price_is_delayed_a_guess_is_refused() {
        assert_eq!(
            place(Some(received(6_400_000)), true),
            Err(Refusal::PriceDelayed)
        );
    }

    #[test]
    fn before_any_price_a_guess_is_refused() {
        assert_eq!(place(None, false), Err(Refusal::PriceDelayed));
    }

    #[test]
    fn a_guess_has_60_seconds_left_when_it_is_placed() {
        let guess = place(Some(received(6_400_000)), false).unwrap();

        assert_eq!(guess.remaining(NOW), Duration::from_secs(60));
    }

    #[test]
    fn the_time_left_counts_down_with_the_servers_clock() {
        let guess = place(Some(received(6_400_000)), false).unwrap();
        let later = NOW
            .checked_add(SignedDuration::from_millis(20_250))
            .unwrap();

        assert_eq!(guess.remaining(later), Duration::from_millis(39_750));
    }

    #[test]
    fn a_due_guess_has_no_time_left() {
        let guess = place(Some(received(6_400_000)), false).unwrap();
        let due = NOW.checked_add(SignedDuration::from_secs(60)).unwrap();
        let past = NOW.checked_add(SignedDuration::from_secs(90)).unwrap();

        assert_eq!(guess.due_at(), due);
        assert_eq!(guess.remaining(due), Duration::ZERO);
        assert_eq!(guess.remaining(past), Duration::ZERO);
    }
}
