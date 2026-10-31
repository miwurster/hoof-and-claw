//! Resolving Guesses on received prices, with a fake Price source, a clock the test moves, the in-memory stores and
//! the recording `Fanout`.

use std::sync::atomic::Ordering;
use std::time::Duration;

use game::{Event, Open, PlayerEvent};
use guessing::{Direction, Outcome, PlayerId, Resolution, Score};
use identity::Token;
use jiff::SignedDuration;
use prices::{Price, Report};
use running::{Running, settle};
use support::Stores;

mod running;
mod support;

fn alice() -> Token {
    Token::from_random_bytes([1; 32])
}

fn bob() -> Token {
    Token::from_random_bytes([2; 32])
}

#[expect(
    clippy::unwrap_used,
    reason = "a test helper fails by panicking, as every test does"
)]
impl Running {
    async fn places(&self, token: &Token, direction: Direction) -> Open {
        self.game.place_guess(token, direction).await.unwrap()
    }

    async fn player(&self, token: &Token) -> PlayerId {
        self.game.player(token).await.unwrap().unwrap()
    }

    fn waits(&self, seconds: i64) {
        self.clock.advance(SignedDuration::from_secs(seconds));
    }

    /// Every resolution published to the Player so far.
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

    fn published_prices(&self) -> Vec<u64> {
        self.fanout
            .published()
            .iter()
            .filter(|latest| !latest.is_delayed())
            .filter_map(|latest| latest.price().map(Price::cents))
            .collect()
    }
}

#[tokio::test(start_paused = true)]
async fn a_guess_resolves_on_the_first_changed_price_received_60_seconds_after_it_was_placed() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    let open = running.places(&alice(), Direction::Up).await;
    let alice_id = running.player(&alice()).await;

    running.waits(30);
    running.receives(6_400_500).await;
    running.waits(30);
    running.receives(6_401_000).await;

    let resolutions = running.resolutions(alice_id);
    assert_eq!(resolutions.len(), 1, "{resolutions:?}");
    let resolution = resolutions[0];
    assert_eq!(resolution.guess.guess, open.guess);
    assert_eq!(resolution.guess.end.price, Price::from_cents(6_401_000));
    assert_eq!(resolution.guess.outcome, Outcome::Correct);
    assert_eq!(resolution.score, Score::new(1));
    let snapshot = running.game.snapshot(Some(alice_id)).await.unwrap();
    assert_eq!(snapshot.score, Score::new(1));
    assert_eq!(snapshot.open_guess, None);
    assert_eq!(snapshot.last_resolved, Some(resolution.guess));
}

#[tokio::test(start_paused = true)]
async fn a_trade_at_the_same_price_as_the_last_one_runs_resolution_but_publishes_no_price() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    running.places(&alice(), Direction::Down).await;
    let alice_id = running.player(&alice()).await;
    running.waits(30);
    tokio::time::sleep(Duration::from_secs(2)).await;
    running.receives(6_399_000).await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    running.waits(30);
    running.receives(6_399_000).await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    settle().await;

    let resolutions = running.resolutions(alice_id);
    assert_eq!(resolutions.len(), 1, "{resolutions:?}");
    assert_eq!(resolutions[0].guess.outcome, Outcome::Correct);
    assert_eq!(running.published_prices(), [6_400_000, 6_399_000]);
}

#[tokio::test(start_paused = true)]
async fn a_resolution_is_published_only_after_its_save_and_a_failed_save_is_retried_with_the_same_end_price()
 {
    let stores = Stores::default();
    let running = Running::start_with(&stores);
    running.live_at(6_400_000).await;
    running.places(&alice(), Direction::Up).await;
    let alice_id = running.player(&alice()).await;
    stores.failing_resolutions.store(2, Ordering::SeqCst);

    running.waits(60);
    running.receives(6_399_000).await;
    running.receives(6_401_000).await;

    assert_eq!(
        running.resolutions(alice_id),
        [],
        "nothing is published while the save fails"
    );

    tokio::time::sleep(Duration::from_secs(10)).await;
    settle().await;

    let resolutions = running.resolutions(alice_id);
    assert_eq!(resolutions.len(), 1, "{resolutions:?}");
    assert_eq!(resolutions[0].guess.end.price, Price::from_cents(6_399_000));
    assert_eq!(resolutions[0].guess.outcome, Outcome::Wrong);
    assert_eq!(resolutions[0].score, Score::new(-1));
    let snapshot = running.game.snapshot(Some(alice_id)).await.unwrap();
    assert_eq!(snapshot.score, Score::new(-1));
}

#[tokio::test(start_paused = true)]
async fn an_open_guess_that_falls_due_while_the_price_is_delayed_resolves_on_the_first_changed_price_after_it_recovers()
 {
    let running = Running::start();
    running.live_at(6_400_000).await;
    running.places(&alice(), Direction::Up).await;
    let alice_id = running.player(&alice()).await;

    running.report(Report::Delayed).await;
    running.waits(120);
    running.report(Report::Live).await;
    running.receives(6_400_000).await;

    assert_eq!(running.resolutions(alice_id), []);

    running.receives(6_400_001).await;

    let resolutions = running.resolutions(alice_id);
    assert_eq!(resolutions.len(), 1, "{resolutions:?}");
    assert_eq!(resolutions[0].guess.end.price, Price::from_cents(6_400_001));
}

#[tokio::test(start_paused = true)]
async fn two_players_do_not_affect_each_other() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    running.places(&bob(), Direction::Down).await;
    let bob_id = running.player(&bob()).await;
    running.waits(30);
    let alices = running.places(&alice(), Direction::Up).await;
    let alice_id = running.player(&alice()).await;

    running.waits(30);
    running.receives(6_401_000).await;

    assert_eq!(running.resolutions(bob_id).len(), 1);
    assert_eq!(running.resolutions(alice_id), []);
    let snapshot = running.game.snapshot(Some(alice_id)).await.unwrap();
    assert_eq!(snapshot.score, Score::new(0));
    assert_eq!(
        snapshot.open_guess.map(|open| open.guess),
        Some(alices.guess)
    );
    assert_eq!(snapshot.last_resolved, None);
}

#[tokio::test(start_paused = true)]
async fn a_tab_whose_queue_is_full_is_closed_and_its_next_snapshot_holds_every_outcome_and_score_it_missed()
 {
    let running = Running::start();
    running.live_at(6_400_000).await;
    running.places(&alice(), Direction::Up).await;
    let alice_id = running.player(&alice()).await;
    let mut slow_tab = running.game.subscribe(Some(alice_id));
    let mut cents = 6_400_000;

    // Each round is a Guess resolved and the next one placed: two Player events, 18 in all.
    for _ in 0..9 {
        running.waits(60);
        cents += 100;
        running.receives(cents).await;
        running.places(&alice(), Direction::Up).await;
    }

    let mut delivered = 0;
    while let Some(event) = slow_tab.next().await {
        if matches!(event, Event::Player(_)) {
            delivered += 1;
        }
    }
    assert_eq!(delivered, game::fanout::PLAYER_EVENT_QUEUE);
    let snapshot = running.game.snapshot(Some(alice_id)).await.unwrap();
    assert_eq!(snapshot.score, Score::new(9));
    assert_eq!(
        snapshot.last_resolved.map(|resolved| resolved.end.price),
        Some(Price::from_cents(cents))
    );
    assert!(snapshot.open_guess.is_some());
}

#[tokio::test(start_paused = true)]
async fn a_stream_opened_before_the_visitors_first_guess_gets_that_players_events() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    let (player, mut tab) = running.game.open_stream(&alice()).await.unwrap();
    assert_eq!(
        player, None,
        "a visitor is no Player before the first Guess"
    );

    let open = running.places(&alice(), Direction::Up).await;
    running.waits(60);
    running.receives(6_400_100).await;

    assert_eq!(
        tab.next().await,
        Some(Event::Player(PlayerEvent::GuessPlaced(open)))
    );
    match tab.next().await {
        Some(Event::Player(PlayerEvent::GuessResolved(resolution))) => {
            assert_eq!(resolution.score, Score::new(1));
        }
        other => panic!("the tab should get the resolution, but got {other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn a_stream_of_a_known_player_opens_as_that_player() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    running.places(&alice(), Direction::Up).await;

    let (player, _tab) = running.game.open_stream(&alice()).await.unwrap();

    assert_eq!(player, Some(running.player(&alice()).await));
}
