//! Continuing across restarts: a new game on the same stores picks up the Open guesses, and a game shutting down
//! stops taking Guesses and prices, with a fake Price source, a clock the test moves, the in-memory stores and the
//! recording `Fanout`.

use std::sync::atomic::Ordering;
use std::time::Duration;

use game::PlaceError;
use guessing::{Direction, Outcome, Refusal, Score};
use prices::Price;
use running::{Running, alice, bob, settle};
use support::Stores;

#[expect(dead_code, reason = "this binary takes only some of the steps")]
mod running;
mod support;

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
    let (_, mut stream) = running.game.open_stream(&alice()).await.unwrap();

    running.game.shut_down();
    running.game.end_streams();

    let delayed = running.fanout.published().last().copied().unwrap();
    assert!(delayed.is_delayed());
    assert_eq!(stream.next().await, Some(game::Event::Price(delayed)));
    assert_eq!(stream.next().await, None);
}

/// What every failure of an unavailable store reads as.
const STORE_DOWN: &str =
    "the game cannot reach its store: the store is unavailable: the store is down";

#[tokio::test(start_paused = true)]
async fn while_the_store_is_down_the_game_reports_it_unavailable() {
    let stores = Stores::default();
    let running = Running::start_with(&stores);
    running.live_at(6_400_000).await;
    running
        .game
        .place_guess(&alice(), Direction::Up)
        .await
        .unwrap();
    let alice_id = running.player(&alice()).await;
    stores.down.store(true, Ordering::SeqCst);

    let rebuilt = running.game.rebuild_due_guesses().await;
    let stream = running.game.open_stream(&alice()).await;
    let snapshot = running.game.snapshot(Some(alice_id)).await;
    let guess = running.game.place_guess(&bob(), Direction::Down).await;

    assert_eq!(rebuilt.unwrap_err().to_string(), STORE_DOWN);
    assert_eq!(stream.err().unwrap().to_string(), STORE_DOWN);
    assert_eq!(snapshot.unwrap_err().to_string(), STORE_DOWN);
    assert!(
        matches!(&guess, Err(PlaceError::Unavailable(_))),
        "{guess:?}"
    );
    assert_eq!(guess.unwrap_err().to_string(), STORE_DOWN);
}
