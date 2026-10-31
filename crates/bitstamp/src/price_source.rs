//! The Bitstamp `PriceSource`: one task keeps the connection healthy and reports what happens.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use prices::{Price, PriceSource, Report, TradeId};
use rustls::ClientConfig;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message as Frame;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};

use crate::backoff::Backoff;
use crate::message::{self, Message};
use crate::seed;

/// Bitstamp's public WebSocket feed (ADR 0004).
pub const FEED_URL: &str = "wss://ws.bitstamp.net";
/// Bitstamp's REST ticker for BTC/USD.
pub const TICKER_URL: &str = "https://www.bitstamp.net/api/v2/ticker/btcusd/";

/// Where Bitstamp is and how the adapter keeps its connection healthy (ADR 0007).
#[derive(Clone, Debug)]
pub struct Config {
    pub feed_url: String,
    pub ticker_url: String,
    /// A heartbeat goes out this often.
    pub heartbeat_interval: Duration,
    /// A heartbeat, and a subscription, must be answered within this time.
    pub heartbeat_timeout: Duration,
    /// After this long without a trade, the adapter reconnects and seeds again.
    pub silence_limit: Duration,
    pub backoff_min: Duration,
    pub backoff_max: Duration,
}

/// Prices from Bitstamp, and every change between live and Price delayed.
pub struct Bitstamp {
    reports: mpsc::UnboundedReceiver<Report>,
}

/// The adapter could not start.
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error("setting up TLS for Bitstamp failed: {0}")]
    Tls(#[from] rustls::Error),
}

/// Closes the connection to Bitstamp; dropping it closes the connection too.
pub struct Shutdown {
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl Bitstamp {
    /// Starts connecting in a task of its own; the price is Price delayed until the subscription succeeds.
    ///
    /// # Errors
    ///
    /// When TLS cannot be set up.
    ///
    /// # Panics
    ///
    /// Outside a `tokio` runtime.
    pub fn start(config: Config) -> Result<(Self, Shutdown), StartError> {
        let tls = tls()?;
        let (sender, reports) = mpsc::unbounded_channel();
        let (stop, stopped) = oneshot::channel();
        let feed = Feed {
            reports: sender,
            live: false,
        };
        let task = tokio::spawn(run(config, tls, feed, stopped));
        Ok((Self { reports }, Shutdown { stop, task }))
    }
}

impl PriceSource for Bitstamp {
    async fn next(&mut self) -> Option<Report> {
        self.reports.recv().await
    }
}

impl Shutdown {
    /// Closes the connection, reports Price delayed, and ends the source once that is done.
    pub async fn close(self) {
        let _ = self.stop.send(());
        let _ = self.task.await;
    }
}

/// Why the price is Price delayed, as the log names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Delay {
    Disconnected,
    NotSubscribed,
    HeartbeatUnanswered,
    Silence,
    ShuttingDown,
}

impl fmt::Display for Delay {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Disconnected => "disconnected",
            Self::NotSubscribed => "not subscribed",
            Self::HeartbeatUnanswered => "heartbeat unanswered",
            Self::Silence => "silence",
            Self::ShuttingDown => "shutting down",
        })
    }
}

/// Reports to the game, and logs every change between live and Price delayed.
struct Feed {
    reports: mpsc::UnboundedSender<Report>,
    live: bool,
}

impl Feed {
    fn live(&mut self) {
        if !self.live {
            self.live = true;
            tracing::info!("price live");
            self.report(Report::Live);
        }
    }

    fn delayed(&mut self, delay: Delay) {
        if self.live {
            self.live = false;
            tracing::warn!(reason = %delay, "price delayed");
            self.report(Report::Delayed);
        }
    }

    fn received(&self, price: Price, trade: Option<TradeId>) {
        self.report(Report::Received(price, trade));
    }

    fn report(&self, report: Report) {
        // The game drops the source only when it stops, and then no report matters any more.
        let _ = self.reports.send(report);
    }
}

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// How following one connection ended.
enum End {
    /// An unplanned disconnect: the next attempt waits for the backoff.
    Lost(Delay),
    /// No trade for the silence limit: reconnect at once.
    Silence,
    /// A planned reconnect opened and subscribed this connection before closing the old one.
    Replaced(Box<Socket>),
    ShutDown,
}

async fn run(
    config: Config,
    tls: Arc<ClientConfig>,
    mut feed: Feed,
    mut stopped: oneshot::Receiver<()>,
) {
    let mut backoff = Backoff::new(config.backoff_min, config.backoff_max);
    let mut replacement = None;
    loop {
        let opened = if let Some(socket) = replacement.take() {
            Ok(socket)
        } else {
            tokio::select! {
                opened = open(&config, &tls) => opened.map(Box::new),
                _ = &mut stopped => return feed.delayed(Delay::ShuttingDown),
            }
        };
        let end = match opened {
            Ok(socket) => {
                backoff.reset();
                feed.live();
                Box::pin(follow(*socket, &config, &tls, &mut feed, &mut stopped)).await
            }
            Err(delay) => End::Lost(delay),
        };
        match end {
            End::Lost(delay) => {
                feed.delayed(delay);
                let wait = backoff.next_wait(getrandom::u32().unwrap_or(u32::MAX));
                tracing::info!(
                    wait_ms = wait.as_millis(),
                    "reconnecting to Bitstamp after {delay}"
                );
                tokio::select! {
                    () = tokio::time::sleep(wait) => {}
                    _ = &mut stopped => return feed.delayed(Delay::ShuttingDown),
                }
            }
            End::Silence => {
                feed.delayed(Delay::Silence);
                tracing::info!("reconnecting to Bitstamp after silence");
            }
            End::Replaced(socket) => replacement = Some(socket),
            End::ShutDown => return feed.delayed(Delay::ShuttingDown),
        }
    }
}

/// TLS with the operating system's root certificates and ring, for the feed and the ticker alike.
fn tls() -> Result<Arc<ClientConfig>, StartError> {
    let found = rustls_native_certs::load_native_certs();
    for error in &found.errors {
        tracing::warn!("loading a root certificate failed: {error}");
    }
    let mut roots = rustls::RootCertStore::empty();
    let (_, unusable) = roots.add_parsable_certificates(found.certs);
    if unusable > 0 {
        tracing::warn!("{unusable} root certificates are unusable");
    }
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// Opens a connection and subscribes to `live_trades_btcusd`; the subscription must succeed within the heartbeat timeout.
async fn open(config: &Config, tls: &Arc<ClientConfig>) -> Result<Socket, Delay> {
    let connected = tokio::time::timeout(
        config.heartbeat_timeout,
        tokio_tungstenite::connect_async_tls_with_config(
            config.feed_url.as_str(),
            None,
            false,
            Some(Connector::Rustls(Arc::clone(tls))),
        ),
    )
    .await;
    let mut socket = match connected {
        Ok(Ok((socket, _))) => socket,
        Ok(Err(error)) => {
            tracing::info!("connecting to Bitstamp failed: {error}");
            return Err(Delay::Disconnected);
        }
        Err(_) => {
            tracing::info!("connecting to Bitstamp timed out");
            return Err(Delay::Disconnected);
        }
    };
    if socket
        .send(Frame::text(message::subscribe()))
        .await
        .is_err()
    {
        return Err(Delay::Disconnected);
    }
    let subscribed = tokio::time::timeout(config.heartbeat_timeout, async {
        while let Some(Ok(frame)) = socket.next().await {
            if let Frame::Text(text) = frame
                && message::parse(&text) == Ok(Message::SubscriptionSucceeded)
            {
                return true;
            }
        }
        false
    })
    .await;
    if subscribed == Ok(true) {
        Ok(socket)
    } else {
        tracing::info!("the subscription to Bitstamp did not succeed");
        let _ = socket.close(None).await;
        Err(Delay::NotSubscribed)
    }
}

/// Follows a subscribed connection: delivers trades and the seed, sends heartbeats, and watches for silence.
async fn follow(
    mut socket: Socket,
    config: &Config,
    tls: &Arc<ClientConfig>,
    feed: &mut Feed,
    stopped: &mut oneshot::Receiver<()>,
) -> End {
    let mut seed = Some(Box::pin(tokio::time::timeout(
        config.heartbeat_timeout,
        seed::read(tls, &config.ticker_url),
    )));
    let mut traded = false;
    let mut heartbeats = tokio::time::interval_at(
        Instant::now() + config.heartbeat_interval,
        config.heartbeat_interval,
    );
    let mut answer_due: Option<Instant> = None;
    let mut silence_ends = Instant::now() + config.silence_limit;
    let end = loop {
        tokio::select! {
            frame = socket.next() => {
                let text = match frame {
                    Some(Ok(Frame::Text(text))) => text,
                    Some(Ok(Frame::Close(_)) | Err(_)) | None => break End::Lost(Delay::Disconnected),
                    Some(Ok(_)) => continue,
                };
                match message::parse(&text) {
                    Ok(Message::Trade(price, trade)) => {
                        traded = true;
                        silence_ends = Instant::now() + config.silence_limit;
                        feed.received(price, trade);
                    }
                    Ok(Message::HeartbeatAnswered) => answer_due = None,
                    Ok(Message::RequestReconnect) => {
                        tracing::info!("Bitstamp requested a reconnect");
                        break match open(config, tls).await {
                            Ok(replacement) => End::Replaced(Box::new(replacement)),
                            Err(delay) => End::Lost(delay),
                        };
                    }
                    Ok(Message::Unsubscribed) => break End::Lost(Delay::NotSubscribed),
                    Ok(Message::SubscriptionSucceeded | Message::Other) => {}
                    Err(error) => tracing::error!("ignoring a trade from Bitstamp: {error}"),
                }
            }
            _ = heartbeats.tick() => {
                if answer_due.is_none() {
                    if socket.send(Frame::text(message::heartbeat())).await.is_err() {
                        break End::Lost(Delay::Disconnected);
                    }
                    answer_due = Some(Instant::now() + config.heartbeat_timeout);
                }
            }
            () = sleep_until(answer_due) => break End::Lost(Delay::HeartbeatUnanswered),
            () = tokio::time::sleep_until(silence_ends) => break End::Silence,
            seeded = async {
                match seed.as_mut() {
                    Some(seed) => seed.await,
                    None => std::future::pending().await,
                }
            }, if seed.is_some() => {
                seed = None;
                match seeded {
                    // A trade received meanwhile is newer than the ticker's `last`.
                    // The seed is no trade, so it has no trade id.
                    Ok(Ok(price)) if !traded => feed.received(price, None),
                    Ok(Ok(_)) => {}
                    Ok(Err(error)) => tracing::warn!("no seed from Bitstamp's ticker: {error}"),
                    Err(_) => tracing::warn!("no seed from Bitstamp's ticker: it did not answer in time"),
                }
            }
            _ = &mut *stopped => break End::ShutDown,
        }
    };
    let _ = socket.close(None).await;
    end
}

async fn sleep_until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}
