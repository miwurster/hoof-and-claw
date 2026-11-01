//! Placing a Guess, with a fake Price source, a clock the test moves, the in-memory stores and the recording `Fanout`.

use std::time::Duration;

use game::{Event, Fanout, PlaceError, PlayerEvent};
use guessing::{Direction, Refusal, Score};
use identity::Token;
use jiff::SignedDuration;
use prices::{Price, ReceivedPrice, Report, Source};
use running::{NOW, Running};

mod running;
mod support;

fn alice() -> Token {
    Token::from_random_bytes([1; 32])
}

fn bob() -> Token {
    Token::from_random_bytes([2; 32])
}

#[tokio::test(start_paused = true)]
async fn a_guess_starts_from_the_last_price_received_with_60_seconds_left() {
    let running = Running::start();
    running.live_at(6_400_000).await;

    let open = running
        .game
        .place_guess(&alice(), Direction::Up)
        .await
        .unwrap();

    assert_eq!(open.guess.direction, Direction::Up);
    assert_eq!(
        open.guess.start,
        ReceivedPrice {
            price: Price::from_cents(6_400_000),
            source: Source::Bitstamp,
            trade_id: None,
            received_at: NOW,
        }
    );
    assert_eq!(open.guess.placed_at, NOW);
    assert_eq!(open.remaining, Duration::from_secs(60));
}

#[tokio::test(start_paused = true)]
async fn the_start_price_is_the_last_price_received_even_before_the_streams_were_sent_it() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    running.clock.advance(SignedDuration::from_millis(300));

    // Within the same second, so the throttle holds it back from the streams.
    running
        .report(Report::Received(Price::from_cents(6_400_050), None))
        .await;
    let open = running
        .game
        .place_guess(&alice(), Direction::Down)
        .await
        .unwrap();

    assert_eq!(open.guess.start.price, Price::from_cents(6_400_050));
    assert_eq!(
        open.guess.start.received_at,
        NOW.checked_add(SignedDuration::from_millis(300)).unwrap()
    );
}

#[tokio::test(start_paused = true)]
async fn while_the_price_is_delayed_a_guess_is_refused_and_nothing_is_stored() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    running.report(Report::Delayed).await;

    let refused = running.game.place_guess(&alice(), Direction::Up).await;

    assert!(
        matches!(refused, Err(PlaceError::Refused(Refusal::PriceDelayed))),
        "{refused:?}"
    );
    assert_eq!(running.game.player(&alice()).await.unwrap(), None);
}

#[tokio::test(start_paused = true)]
async fn before_any_price_a_guess_is_refused() {
    let running = Running::start();
    running.report(Report::Live).await;

    let refused = running.game.place_guess(&alice(), Direction::Up).await;

    assert!(
        matches!(refused, Err(PlaceError::Refused(Refusal::PriceDelayed))),
        "{refused:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn a_player_with_an_open_guess_cannot_place_another() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    let first = running
        .game
        .place_guess(&alice(), Direction::Up)
        .await
        .unwrap();

    let second = running.game.place_guess(&alice(), Direction::Down).await;

    assert!(
        matches!(second, Err(PlaceError::Refused(Refusal::OpenGuessExists))),
        "{second:?}"
    );
    let player = running.game.player(&alice()).await.unwrap();
    let snapshot = running.game.snapshot(player).await.unwrap();
    assert_eq!(
        snapshot.open_guess.map(|open| open.guess),
        Some(first.guess)
    );
}

#[tokio::test(start_paused = true)]
async fn a_visitor_reads_as_score_0_with_no_open_guess_and_is_stored_by_nothing() {
    let running = Running::start();
    running.live_at(6_400_000).await;

    let player = running.game.player(&alice()).await.unwrap();
    let snapshot = running.game.snapshot(player).await.unwrap();

    assert_eq!(player, None);
    assert_eq!(snapshot.score, Score::new(0));
    assert_eq!(snapshot.open_guess, None);
    assert_eq!(running.game.player(&alice()).await.unwrap(), None);
}

#[tokio::test(start_paused = true)]
async fn the_snapshot_counts_the_open_guess_down_with_the_servers_clock() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    let placed = running
        .game
        .place_guess(&alice(), Direction::Up)
        .await
        .unwrap();

    running.clock.advance(SignedDuration::from_secs(20));
    let player = running.game.player(&alice()).await.unwrap();
    let snapshot = running.game.snapshot(player).await.unwrap();

    let open = snapshot.open_guess.unwrap();
    assert_eq!(open.guess, placed.guess);
    assert_eq!(open.remaining, Duration::from_secs(40));
    assert_eq!(snapshot.score, Score::new(0));
}

#[tokio::test(start_paused = true)]
async fn a_placed_guess_is_published_as_guess_placed_to_its_player_and_a_refused_one_is_not() {
    let running = Running::start();
    running.live_at(6_400_000).await;

    let bobs = running
        .game
        .place_guess(&bob(), Direction::Down)
        .await
        .unwrap();
    let alices = running
        .game
        .place_guess(&alice(), Direction::Up)
        .await
        .unwrap();
    let refused = running.game.place_guess(&alice(), Direction::Down).await;

    assert!(refused.is_err());
    let bob_id = running.game.player(&bob()).await.unwrap().unwrap();
    let alice_id = running.game.player(&alice()).await.unwrap().unwrap();
    assert_eq!(
        running.fanout.notified(),
        [
            (bob_id, PlayerEvent::GuessPlaced(bobs)),
            (alice_id, PlayerEvent::GuessPlaced(alices)),
        ]
    );
}

#[tokio::test(start_paused = true)]
async fn the_same_browser_stays_the_same_player() {
    let running = Running::start();
    running.live_at(6_400_000).await;

    running
        .game
        .place_guess(&alice(), Direction::Up)
        .await
        .unwrap();
    let first = running.game.player(&alice()).await.unwrap();
    let again = running.game.player(&alice()).await.unwrap();

    assert!(first.is_some());
    assert_eq!(first, again);
    assert_ne!(first, running.game.player(&bob()).await.unwrap());
}

#[tokio::test(start_paused = true)]
async fn a_price_still_reaches_a_players_stream() {
    let running = Running::start();
    running.live_at(6_400_000).await;
    running
        .game
        .place_guess(&alice(), Direction::Up)
        .await
        .unwrap();
    let alice_id = running.game.player(&alice()).await.unwrap();
    let mut stream = running.game.subscribe(alice_id);

    running.report(Report::Delayed).await;

    match stream.next().await {
        Some(Event::Price(latest)) => assert!(latest.is_delayed()),
        other => panic!("expected a price, got {other:?}"),
    }
}

#[tokio::test(start_paused = true)]
async fn a_stream_opened_while_the_first_guess_is_placed_gets_the_players_events() {
    let stores = support::Stores::default();
    let running = Running::start_with(&stores);
    running.live_at(6_400_000).await;
    let (_, _first_tab) = running.game.open_stream(&alice()).await.unwrap();
    // The second tab has read that the token is no Player yet when the first Guess links it.
    let (held, held_now) = tokio::sync::oneshot::channel();
    let (go_on, going_on) = tokio::sync::oneshot::channel();
    *stores.held_player_read.lock().unwrap() = Some((held, going_on));
    let second_tab = tokio::spawn({
        let game = running.game.clone();
        async move { game.open_stream(&alice()).await }
    });
    held_now.await.unwrap();
    let open = running
        .game
        .place_guess(&alice(), Direction::Up)
        .await
        .unwrap();
    go_on.send(()).unwrap();
    let (player, mut stream) = second_tab.await.unwrap().unwrap();

    let alice_id = open.guess.player;
    assert_eq!(player, Some(alice_id));
    running
        .fanout
        .notify_player(alice_id, PlayerEvent::GuessPlaced(open));
    let event = tokio::time::timeout(Duration::from_secs(1), stream.next()).await;
    assert_eq!(
        event,
        Ok(Some(Event::Player(PlayerEvent::GuessPlaced(open)))),
        "the second tab should get Alice's events"
    );
}
