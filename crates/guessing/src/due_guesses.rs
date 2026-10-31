use std::collections::BTreeMap;

use jiff::Timestamp;
use prices::ReceivedPrice;

use crate::{GuessId, OpenGuess, ResolvedGuess};

/// The index of Open guesses by when they fall due, which resolution runs over on every received price (ADR 0010).
#[derive(Clone, Debug, Default)]
pub struct DueGuesses {
    by_due_time: BTreeMap<(Timestamp, GuessId), OpenGuess>,
}

impl DueGuesses {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an Open guess, which then waits until it falls due and the price changes.
    pub fn add(&mut self, guess: OpenGuess) {
        self.by_due_time.insert((guess.due_at(), guess.id), guess);
    }

    /// Takes out every Open guess that the received price resolves, with that price as its End price.
    ///
    /// A Guess that is due but whose Start price equals the received price stays in the index.
    pub fn resolve(&mut self, received: ReceivedPrice) -> Vec<ResolvedGuess> {
        let resolved: Vec<ResolvedGuess> = self
            .by_due_time
            .values()
            .take_while(|guess| guess.due_at() <= received.received_at)
            .filter_map(|guess| guess.resolve(received))
            .collect();
        for resolution in &resolved {
            self.by_due_time
                .remove(&(resolution.guess.due_at(), resolution.guess.id));
        }
        resolved
    }
}

#[cfg(test)]
mod tests {
    use jiff::SignedDuration;
    use prices::{Price, Source};

    use super::*;
    use crate::{Direction, PlayerId};

    const NOW: Timestamp = Timestamp::constant(1_798_761_600, 0);

    fn at(seconds: i64) -> Timestamp {
        NOW.checked_add(SignedDuration::from_secs(seconds)).unwrap()
    }

    fn received(cents: u64, seconds: i64) -> ReceivedPrice {
        ReceivedPrice {
            price: Price::from_cents(cents),
            source: Source::Bitstamp,
            trade_id: None,
            received_at: at(seconds),
        }
    }

    fn guess(byte: u8, placed_at: i64, start: u64) -> OpenGuess {
        OpenGuess {
            id: GuessId::new(at(placed_at), [byte; 10]),
            player: PlayerId::new(at(placed_at), [byte; 10]),
            direction: Direction::Up,
            start: received(start, placed_at),
            placed_at: at(placed_at),
        }
    }

    fn resolved_ids(resolved: &[ResolvedGuess]) -> Vec<GuessId> {
        resolved
            .iter()
            .map(|resolution| resolution.guess.id)
            .collect()
    }

    #[test]
    fn a_price_resolves_only_the_guesses_that_are_due() {
        let mut index = DueGuesses::new();
        let early = guess(1, 0, 6_400_000);
        let late = guess(2, 30, 6_400_000);
        index.add(early);
        index.add(late);

        let resolved = index.resolve(received(6_400_100, 60));

        assert_eq!(resolved_ids(&resolved), [early.id]);
        assert_eq!(resolved[0].end, received(6_400_100, 60));
    }

    #[test]
    fn a_due_guess_whose_start_price_equals_the_price_stays_until_the_price_changes() {
        let mut index = DueGuesses::new();
        let unchanged = guess(1, 0, 6_400_100);
        let changed = guess(2, 0, 6_400_000);
        index.add(unchanged);
        index.add(changed);

        let first = index.resolve(received(6_400_100, 60));
        let second = index.resolve(received(6_400_200, 61));

        assert_eq!(resolved_ids(&first), [changed.id]);
        assert_eq!(resolved_ids(&second), [unchanged.id]);
    }

    #[test]
    fn a_resolved_guess_leaves_the_index() {
        let mut index = DueGuesses::new();
        index.add(guess(1, 0, 6_400_000));
        index.resolve(received(6_400_100, 60));

        assert!(index.resolve(received(6_400_200, 61)).is_empty());
    }
}
