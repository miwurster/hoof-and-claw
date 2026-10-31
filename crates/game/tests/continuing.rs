//! Continuing across restarts: a new game on the same stores picks up the Open guesses, and a game shutting down
//! stops taking Guesses and prices, with a fake Price source, a clock the test moves, the in-memory stores and the
//! recording `Fanout`.

use std::sync::atomic::Ordering;
use std::time::Duration;

use game::{PlaceError, PlayerEvent};
use guessing::{Direction, Outcome, PlayerId, Refusal, Resolution, Score};
use identity::Token;
use jiff::SignedDuration;
use prices::Price;
use running::{Running, settle};
use support::Stores;

mod running;
mod support;

fn alice() -> Token {
    Token::from_random_bytes([1; 32])
}

#[expect(
    clippy::unwrap_used,
    reason = "a test helper fails by panicking, as every test does"
)]
impl Running {
    async fn player(&self, token: &Token) -> PlayerId {
        self.game.player(token).await.unwrap().unwrap()
    }

    fn waits(&self, seconds: i64) {
        self.clock.advance(SignedDuration::from_secs(seconds));
    }

    fn resolutions(&self, player: PlayerId) -> Vec<Resolution> {
        self.fanout
            .notified()
            .into_iter()
            .filter(|(whose, _)| *whose == player)
            .filter_map(|(_, event)| match event {
                PlayerEvent::GuessResolved(resolution) => Some(resolution),
                PlayerEvent::GuessPlaced(_) => None,
            })
            .collect()
    }
}

#[tokio::test(start_paused = true)]
async fn an_open_guess_survives_a_restart_and_resolves_on_the_first_changed_price_after_it() {
    let stores = Stores::default();
    let before = Running::start_with(&stores);
    before.live_at(6_400_000).await;
    let open = before
        .game
        .place_guess(&alice(), Direction::Up)
        .await
        .unwrap();
    let alice_id = before.player(&alice()).await;

    let after = Running::start_with(&stores);
    let waiting = after.game.rebuild_due_guesses().await.unwrap();
    after.waits(60);
    after.live_at(6_400_000).await;
    after.receives(6_400_100).await;

    assert_eq!(waiting, 1, "one Open guess should wait for resolution");
    let resolutions = after.resolutions(alice_id);
    assert_eq!(resolutions.len(), 1, "{resolutions:?}");
    assert_eq!(resolutions[0].guess.guess, open.guess);
    assert_eq!(resolutions[0].guess.end.price, Price::from_cents(6_400_100));
    assert_eq!(resolutions[0].guess.outcome, Outcome::Correct);
    assert_eq!(resolutions[0].score, Score::new(1));
}

#[tokio::test(start_paused = true)]
async fn a_game_shutting_down_publishes_price_delayed_and_refuses_guesses_with_price_delayed() {
    let running = Running::start();
    running.live_at(6_400_000).await;

    running.game.shut_down();
    let refused = running.game.place_guess(&alice(), Direction::Up).await;

    assert!(
        matches!(refused, Err(PlaceError::Refused(Refusal::PriceDelayed))),
        "the Guess should be refused as Price delayed, but was {refused:?}"
    );
    let last = running.fanout.published().last().copied().unwrap();
    assert!(
        last.is_delayed(),
        "the last price published should be delayed"
    );
    assert_eq!(last.price(), Some(Price::from_cents(6_400_000)));
}

#[tokio::test(start_paused = true)]
async fn a_game_shutting_down_stops_taking_prices() {
    let running = Running::start();
    running.live_at(6_400_000).await;

    running.game.shut_down();
    settle().await;

    assert!(
        running.task.is_finished(),
        "the game should take no more prices"
    );
}

#[tokio::test(start_paused = true)]
async fn a_game_shutting_down_waits_for_a_running_save_of_resolutions() {
    let stores = Stores::default();
    let running = Running::start_with(&stores);
    running.live_at(6_400_000).await;
    running
        .game
        .place_guess(&alice(), Direction::Up)
        .await
        .unwrap();
    let alice_id = running.player(&alice()).await;
    stores.failing_resolutions.store(2, Ordering::SeqCst);
    running.waits(60);
    running.receives(6_400_100).await;

    running.game.shut_down();
    settle().await;

    assert!(
        !running.task.is_finished(),
        "the game should wait while the save is retried"
    );
    tokio::time::sleep(Duration::from_secs(1)).await;
    settle().await;
    assert!(
        running.task.is_finished(),
        "the game should stop once saved"
    );
    assert_eq!(running.resolutions(alice_id).len(), 1);
}

#[tokio::test(start_paused = true)]
async fn a_stream_open_while_the_game_shuts_down_gets_price_delayed_before_it_ends() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    let mut stream = running.game.subscribe(None);

    running.game.shut_down();
    running.game.end_streams();

    let mut delayed = running.fanout.published().last().copied().unwrap();
    assert!(delayed.is_delayed());
    delayed.apply(prices::Report::Delayed);
    assert_eq!(stream.next().await, Some(game::Event::Price(delayed)));
    assert_eq!(stream.next().await, None);
}
