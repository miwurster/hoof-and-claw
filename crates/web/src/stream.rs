//! The game's routes under `/api/`: the snapshot and the stream of events.

use std::convert::Infallible;
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::{Stream, StreamExt as _, stream};
use game::{Clock, Fanout, Game};

use crate::event::{PriceChanged, ServerEvent, Snapshot};

/// What the game's routes share.
pub(crate) struct Api<C, F> {
    pub(crate) game: Game<C, F>,
    pub(crate) build: String,
    pub(crate) keepalive: Duration,
}

impl<C: Clone, F: Clone> Clone for Api<C, F> {
    fn clone(&self) -> Self {
        Self {
            game: self.game.clone(),
            build: self.build.clone(),
            keepalive: self.keepalive,
        }
    }
}

impl<C: Clock, F: Fanout> Api<C, F> {
    /// A visitor without a stored Player reads as Score 0 with no Open guess (ADR 0001).
    fn snapshot(&self) -> Snapshot {
        let snapshot = self.game.snapshot();
        let price = PriceChanged::from(snapshot.latest);
        Snapshot {
            build: self.build.clone(),
            server_time: snapshot.server_time.to_string(),
            price: price.price,
            delayed: price.delayed,
            score: 0,
            open_guess: None,
            last_resolved: None,
        }
    }
}

pub(crate) async fn snapshot<C: Clock, F: Fanout>(
    State(api): State<Api<C, F>>,
) -> Json<ServerEvent> {
    Json(ServerEvent::Snapshot(api.snapshot()))
}

/// One stream per tab: the snapshot first, then every published price, with a comment as keepalive (ADR 0010).
pub(crate) async fn events<C: Clock, F: Fanout>(
    State(api): State<Api<C, F>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Subscribe before the snapshot, so a price published in between arrives as an event (ADR 0010).
    let subscription = api.game.subscribe();
    let first = stream::once(std::future::ready(ServerEvent::Snapshot(api.snapshot())));
    let prices = stream::unfold(subscription, |mut subscription| async move {
        let latest = subscription.next().await?;
        Some((ServerEvent::Price(latest.into()), subscription))
    });
    let events = first
        .chain(prices)
        .filter_map(|event| std::future::ready(Event::default().json_data(event).ok().map(Ok)));
    Sse::new(events).keep_alive(KeepAlive::new().interval(api.keepalive))
}
