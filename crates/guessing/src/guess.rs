use std::time::Duration;

use jiff::Timestamp;
use prices::ReceivedPrice;

use crate::{Direction, GuessId, Outcome, PlayerId};

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

/// A Guess for which at least 60 seconds have passed and the price has changed since it was placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedGuess {
    /// The Guess as it was while open.
    pub guess: OpenGuess,
    /// The first changed price received at least 60 seconds after the Guess (ADR 0002).
    pub end: ReceivedPrice,
    pub outcome: Outcome,
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
    /// Places a Guess at `now` from the live price, the last price received while the price is not delayed (ADR 0007).
    ///
    /// # Errors
    ///
    /// Refuses with `PriceDelayed` without a live price: while the price is delayed or before any price was received.
    pub fn place(
        id: GuessId,
        player: PlayerId,
        direction: Direction,
        live: Option<ReceivedPrice>,
        now: Timestamp,
    ) -> Result<Self, Refusal> {
        let start = live.ok_or(Refusal::PriceDelayed)?;
        Ok(Self {
            id,
            player,
            direction,
            start,
            placed_at: now,
        })
    }

    /// When the Guess falls due: `GUESS_DURATION` after it was placed.
    #[must_use]
    pub fn due_at(&self) -> Timestamp {
        self.placed_at
            .saturating_add(GUESS_DURATION)
            .unwrap_or(Timestamp::MAX)
    }

    /// Resolves the Guess against a received price, or `None` while it stays open: before it falls due, at a price
    /// equal to the Start price, or at a price from another Price source (ADR 0002).
    #[must_use]
    pub fn resolve(&self, received: ReceivedPrice) -> Option<ResolvedGuess> {
        let counts = received.received_at >= self.due_at()
            && received.source == self.start.source
            && received.price != self.start.price;
        if !counts {
            return None;
        }
        let went_up = received.price > self.start.price;
        let outcome = if went_up == (self.direction == Direction::Up) {
            Outcome::Correct
        } else {
            Outcome::Wrong
        };
        Some(ResolvedGuess {
            guess: *self,
            end: received,
            outcome,
        })
    }

    /// The time left at `now` until the Guess falls due, 0 once it has.
    #[must_use]
    pub fn remaining(&self, now: Timestamp) -> Duration {
        Duration::try_from(self.due_at().duration_since(now)).unwrap_or(Duration::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use jiff::SignedDuration;
    use prices::{Price, Source};

    use super::*;

    const NOW: Timestamp = Timestamp::constant(1_798_761_600, 0);

    fn received(cents: u64) -> ReceivedPrice {
        ReceivedPrice {
            price: Price::from_cents(cents),
            source: Source::Bitstamp,
            trade_id: None,
            received_at: NOW,
        }
    }

    fn place(live: Option<ReceivedPrice>) -> Result<OpenGuess, Refusal> {
        OpenGuess::place(
            GuessId::new(NOW, [1; 10]),
            PlayerId::new(NOW, [2; 10]),
            Direction::Up,
            live,
            NOW,
        )
    }

    #[test]
    fn a_guess_starts_from_the_last_price_received() {
        let guess = place(Some(received(6_400_050))).unwrap();

        assert_eq!(guess.start, received(6_400_050));
        assert_eq!(guess.direction, Direction::Up);
        assert_eq!(guess.placed_at, NOW);
    }

    #[test]
    fn without_a_live_price_a_guess_is_refused() {
        assert_eq!(place(None), Err(Refusal::PriceDelayed));
    }

    #[test]
    fn a_guess_has_60_seconds_left_when_it_is_placed() {
        let guess = place(Some(received(6_400_000))).unwrap();

        assert_eq!(guess.remaining(NOW), Duration::from_secs(60));
    }

    #[test]
    fn the_time_left_counts_down_with_the_servers_clock() {
        let guess = place(Some(received(6_400_000))).unwrap();
        let later = NOW
            .checked_add(SignedDuration::from_millis(20_250))
            .unwrap();

        assert_eq!(guess.remaining(later), Duration::from_millis(39_750));
    }

    fn received_at(cents: u64, after: SignedDuration) -> ReceivedPrice {
        ReceivedPrice {
            price: Price::from_cents(cents),
            source: Source::Bitstamp,
            trade_id: None,
            received_at: NOW.checked_add(after).unwrap(),
        }
    }

    fn placed(direction: Direction) -> OpenGuess {
        OpenGuess::place(
            GuessId::new(NOW, [1; 10]),
            PlayerId::new(NOW, [2; 10]),
            direction,
            Some(received(6_400_000)),
            NOW,
        )
        .unwrap()
    }

    #[test]
    fn a_guess_does_not_resolve_before_60_seconds_even_when_the_price_changes() {
        let guess = placed(Direction::Up);

        assert_eq!(
            guess.resolve(received_at(6_400_100, SignedDuration::from_secs(30))),
            None
        );
    }

    #[test]
    fn a_price_received_59_999_seconds_after_the_guess_does_not_resolve_it() {
        let guess = placed(Direction::Up);

        assert_eq!(
            guess.resolve(received_at(6_400_100, SignedDuration::from_millis(59_999))),
            None
        );
    }

    #[test]
    fn a_changed_price_received_at_exactly_60_seconds_resolves_it() {
        let guess = placed(Direction::Up);
        let end = received_at(6_400_100, SignedDuration::from_secs(60));

        let resolved = guess.resolve(end).unwrap();

        assert_eq!(resolved.end, end);
        assert_eq!(resolved.guess, guess);
    }

    #[test]
    fn a_price_equal_to_the_start_price_leaves_the_guess_open_after_60_seconds() {
        let guess = placed(Direction::Down);

        assert_eq!(
            guess.resolve(received_at(6_400_000, SignedDuration::from_secs(90))),
            None
        );
    }

    #[test]
    fn a_difference_of_one_cent_resolves_the_guess() {
        let guess = placed(Direction::Down);

        let resolved = guess
            .resolve(received_at(6_399_999, SignedDuration::from_secs(61)))
            .unwrap();

        assert_eq!(resolved.end.price, Price::from_cents(6_399_999));
    }

    #[test]
    fn the_outcome_follows_the_direction_and_the_way_the_price_went() {
        let later = SignedDuration::from_secs(60);
        let cases = [
            (Direction::Up, 6_400_001, Outcome::Correct, 1),
            (Direction::Down, 6_399_999, Outcome::Correct, 1),
            (Direction::Up, 6_399_999, Outcome::Wrong, -1),
            (Direction::Down, 6_400_001, Outcome::Wrong, -1),
        ];

        for (direction, end, outcome, change) in cases {
            let resolved = placed(direction).resolve(received_at(end, later)).unwrap();

            assert_eq!(resolved.outcome, outcome, "{direction:?} to {end}");
            assert_eq!(
                resolved.outcome.score_change(),
                change,
                "{direction:?} to {end}"
            );
        }
    }

    #[test]
    fn a_due_guess_has_no_time_left() {
        let guess = place(Some(received(6_400_000))).unwrap();
        let due = NOW.checked_add(SignedDuration::from_secs(60)).unwrap();
        let past = NOW.checked_add(SignedDuration::from_secs(90)).unwrap();

        assert_eq!(guess.due_at(), due);
        assert_eq!(guess.remaining(due), Duration::ZERO);
        assert_eq!(guess.remaining(past), Duration::ZERO);
    }
}
