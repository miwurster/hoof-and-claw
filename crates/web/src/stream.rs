//! The game's routes under `/api/` that read: the snapshot and the stream of events.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use axum::Json;
use axum::extract::{ConnectInfo, Extension, State};
use axum::http::HeaderMap;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::{Stream, StreamExt as _, stream};
use game::{Clock, Fanout, Game, Store, Unavailable};
use guessing::PlayerId;
use identity::Token;

use crate::ErrorCode;
use crate::client_ip::ClientIp;
use crate::event::{PriceChanged, ServerEvent, Snapshot};
use crate::streams_per_ip::StreamsPerIp;

/// What the game's routes share.
pub(crate) struct Api<C, F, S> {
    pub(crate) game: Game<C, F, S>,
    pub(crate) build: String,
    pub(crate) keepalive: Duration,
    pub(crate) streams: StreamsPerIp,
    pub(crate) client_ip: ClientIp,
}

impl<C: Clone, F: Clone, S: Clone> Clone for Api<C, F, S> {
    fn clone(&self) -> Self {
        Self {
            game: self.game.clone(),
            build: self.build.clone(),
            keepalive: self.keepalive,
            streams: self.streams.clone(),
            client_ip: self.client_ip.clone(),
        }
    }
}

fn internal(error: &Unavailable) -> ErrorCode {
    tracing::error!("cannot read a Player: {error}");
    ErrorCode::Internal
}

impl<C: Clock, F: Fanout, S: Store> Api<C, F, S> {
    async fn player(&self, token: &Token) -> Result<Option<PlayerId>, ErrorCode> {
        self.game
            .player(token)
            .await
            .map_err(|error| internal(&error))
    }

    /// A visitor without a stored Player reads as Score 0 with no Open guess (ADR 0001).
    async fn snapshot(&self, player: Option<PlayerId>) -> Result<Snapshot, ErrorCode> {
        let snapshot = self
            .game
            .snapshot(player)
            .await
            .map_err(|error| internal(&error))?;
        let price = PriceChanged::from(snapshot.latest);
        Ok(Snapshot {
            build: self.build.clone(),
            server_time: snapshot.server_time.to_string(),
            price: price.price,
            delayed: price.delayed,
            score: snapshot.score.value(),
            open_guess: snapshot.open_guess.map(Into::into),
            last_resolved: snapshot.last_resolved.map(Into::into),
        })
    }
}

pub(crate) async fn snapshot<C: Clock, F: Fanout, S: Store>(
    State(api): State<Api<C, F, S>>,
    Extension(token): Extension<Token>,
) -> Result<Json<ServerEvent>, ErrorCode> {
    let player = api.player(&token).await?;
    Ok(Json(ServerEvent::Snapshot(api.snapshot(player).await?)))
}

/// One stream per tab: the snapshot first, then every published price and the Player's events, with a comment as
/// keepalive (ADR 0010).
///
/// An IP that already holds `MAX_STREAMS_PER_IP` open streams is refused with `rateLimited`.
pub(crate) async fn events<C: Clock, F: Fanout, S: Store>(
    State(api): State<Api<C, F, S>>,
    Extension(token): Extension<Token>,
    connect_info: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ErrorCode> {
    let peer = connect_info.map(|Extension(ConnectInfo(peer))| peer);
    // The place stays taken for as long as the stream below holds it, so it is freed when the tab disconnects. A
    // request without any address is not capped, as with the cookie limit.
    let place = match api.client_ip.of(&headers, peer) {
        Some(ip) => Some(api.streams.open(ip).ok_or(ErrorCode::RateLimited)?),
        None => None,
    };
    // Subscribe before the snapshot, so an event published in between arrives as an event (ADR 0010).
    let (player, subscription) = api
        .game
        .open_stream(&token)
        .await
        .map_err(|error| internal(&error))?;
    let first = stream::once(std::future::ready(ServerEvent::Snapshot(
        api.snapshot(player).await?,
    )));
    let later = stream::unfold(
        (subscription, place),
        |(mut subscription, place)| async move {
            let event = subscription.next().await?;
            Some((ServerEvent::from(event), (subscription, place)))
        },
    );
    let events = first
        .chain(later)
        .filter_map(|event| std::future::ready(Event::default().json_data(event).ok().map(Ok)));
    Ok(Sse::new(events).keep_alive(KeepAlive::new().interval(api.keepalive)))
}
