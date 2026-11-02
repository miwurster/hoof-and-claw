//! A local fake of Bitstamp: the WebSocket feed on one port and the REST ticker on another.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

/// How long a test waits for the fake to reach an expected state.
const PATIENCE: Duration = Duration::from_secs(5);

/// What happened on the fake, in order, for tests about the order of connections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Happened {
    Opened(usize),
    Subscribed(usize),
    Closed(usize),
}

#[derive(Clone)]
pub struct FakeBitstamp {
    feed_port: u16,
    ticker_port: u16,
    state: Arc<Mutex<State>>,
}

struct State {
    connections: Vec<Connection>,
    opened: usize,
    happened: Vec<Happened>,
    confirms_subscriptions: bool,
    answers_heartbeats: bool,
    heartbeats: usize,
    ticker_last: Option<String>,
    ticker_reads: usize,
}

struct Connection {
    number: usize,
    subscribed: bool,
    outgoing: mpsc::UnboundedSender<Outgoing>,
}

enum Outgoing {
    Send(String),
    Close,
}

impl FakeBitstamp {
    /// A fake that confirms subscriptions, answers heartbeats, and has no ticker price yet.
    pub async fn start() -> Self {
        let feed = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ticker = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let fake = Self {
            feed_port: feed.local_addr().unwrap().port(),
            ticker_port: ticker.local_addr().unwrap().port(),
            state: Arc::new(Mutex::new(State {
                connections: Vec::new(),
                opened: 0,
                happened: Vec::new(),
                confirms_subscriptions: true,
                answers_heartbeats: true,
                heartbeats: 0,
                ticker_last: None,
                ticker_reads: 0,
            })),
        };
        tokio::spawn(fake.clone().accept_feed(feed));
        tokio::spawn(fake.clone().serve_ticker(ticker));
        fake
    }

    pub fn feed_url(&self) -> String {
        format!("ws://127.0.0.1:{}", self.feed_port)
    }

    pub fn ticker_url(&self) -> String {
        format!(
            "http://127.0.0.1:{}/api/v2/ticker/btcusd/",
            self.ticker_port
        )
    }

    /// The REST ticker answers with this `last`, or with an error for `None`.
    pub fn set_ticker(&self, last: Option<&str>) {
        self.lock().ticker_last = last.map(str::to_owned);
    }

    pub fn ticker_reads(&self) -> usize {
        self.lock().ticker_reads
    }

    pub fn heartbeats(&self) -> usize {
        self.lock().heartbeats
    }

    pub fn opened(&self) -> usize {
        self.lock().opened
    }

    pub fn happened(&self) -> Vec<Happened> {
        self.lock().happened.clone()
    }

    pub fn answers_heartbeats(&self, answers: bool) {
        self.lock().answers_heartbeats = answers;
    }

    pub fn confirms_subscriptions(&self, confirms: bool) {
        self.lock().confirms_subscriptions = confirms;
    }

    /// A trade on `live_trades_btcusd`, to every subscribed connection.
    pub fn trades(&self, id: u64, price_str: &str) {
        let price: f64 = price_str.parse().unwrap_or(0.0);
        self.to_subscribed(&json!({
            "event": "trade",
            "channel": "live_trades_btcusd",
            "data": {
                "id": id,
                "timestamp": "1793448000",
                "amount": 0.01,
                "amount_str": "0.01000000",
                "price": price,
                "price_str": price_str,
                "type": 0,
                "microtimestamp": "1793448000000000",
                "buy_order_id": 1,
                "sell_order_id": 2
            }
        }));
    }

    /// Bitstamp's warning before maintenance.
    pub fn requests_reconnect(&self) {
        self.to_subscribed(&json!({ "event": "bts:request_reconnect", "channel": "", "data": "" }));
    }

    /// Closes every open connection from Bitstamp's side with a plain close frame, as an unplanned disconnect does.
    pub fn drops_connections(&self) {
        for connection in &self.lock().connections {
            let _ = connection.outgoing.send(Outgoing::Close);
        }
    }

    pub async fn wait_for_opened(&self, count: usize) {
        self.wait_until(&format!("{count} opened connections"), |state| {
            state.opened >= count
        })
        .await;
    }

    pub async fn wait_for_a_subscribed_connection(&self) {
        self.wait_until("a subscribed connection", |state| {
            state
                .connections
                .iter()
                .any(|connection| connection.subscribed)
        })
        .await;
    }

    pub async fn wait_for_no_connection(&self) {
        self.wait_until("no open connection", |state| state.connections.is_empty())
            .await;
    }

    pub async fn wait_for_heartbeats(&self, count: usize) {
        self.wait_until(&format!("{count} heartbeats"), |state| {
            state.heartbeats >= count
        })
        .await;
    }

    async fn wait_until(&self, what: &str, done: impl Fn(&State) -> bool) {
        let waited = tokio::time::timeout(PATIENCE, async {
            while !done(&self.lock()) {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await;
        assert!(waited.is_ok(), "the fake Bitstamp should have seen {what}");
    }

    fn to_subscribed(&self, message: &Value) {
        for connection in self
            .lock()
            .connections
            .iter()
            .filter(|connection| connection.subscribed)
        {
            let _ = connection
                .outgoing
                .send(Outgoing::Send(message.to_string()));
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }

    async fn accept_feed(self, listener: TcpListener) {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(self.clone().serve_connection(stream));
        }
    }

    async fn serve_connection(self, stream: TcpStream) {
        let Ok(mut socket) = tokio_tungstenite::accept_async(stream).await else {
            return;
        };
        let (outgoing, mut commands) = mpsc::unbounded_channel();
        let number = {
            let mut state = self.lock();
            state.opened += 1;
            let number = state.opened;
            state.happened.push(Happened::Opened(number));
            state.connections.push(Connection {
                number,
                subscribed: false,
                outgoing,
            });
            number
        };
        loop {
            tokio::select! {
                incoming = socket.next() => {
                    let Some(Ok(Message::Text(text))) = incoming else { break };
                    let Some(answer) = self.answer(number, &text) else { continue };
                    if socket.send(Message::text(answer.to_string())).await.is_err() {
                        break;
                    }
                }
                command = commands.recv() => match command {
                    Some(Outgoing::Send(text)) => {
                        if socket.send(Message::text(text)).await.is_err() {
                            break;
                        }
                    }
                    Some(Outgoing::Close) | None => {
                        let _ = socket.close(None).await;
                        break;
                    }
                },
            }
        }
        let mut state = self.lock();
        state
            .connections
            .retain(|connection| connection.number != number);
        state.happened.push(Happened::Closed(number));
    }

    fn answer(&self, number: usize, text: &str) -> Option<Value> {
        let message: Value = serde_json::from_str(text).ok()?;
        let mut state = self.lock();
        let channel = message.pointer("/data/channel").and_then(Value::as_str);
        match message.get("event").and_then(Value::as_str)? {
            "bts:subscribe"
                if channel == Some("live_trades_btcusd") && state.confirms_subscriptions =>
            {
                state.happened.push(Happened::Subscribed(number));
                if let Some(connection) = state
                    .connections
                    .iter_mut()
                    .find(|connection| connection.number == number)
                {
                    connection.subscribed = true;
                }
                Some(
                    json!({ "event": "bts:subscription_succeeded", "channel": "live_trades_btcusd", "data": {} }),
                )
            }
            "bts:heartbeat" => {
                state.heartbeats += 1;
                state
                    .answers_heartbeats
                    .then(|| json!({ "event": "bts:heartbeat", "channel": "", "data": { "status": "success" } }))
            }
            _ => None,
        }
    }

    async fn serve_ticker(self, listener: TcpListener) {
        while let Ok((mut stream, _)) = listener.accept().await {
            let fake = self.clone();
            tokio::spawn(async move {
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.ends_with(b"\r\n\r\n") {
                    match stream.read(&mut buffer).await {
                        Ok(0) | Err(_) => return,
                        Ok(read) => {
                            request.extend_from_slice(buffer.get(..read).unwrap_or_default());
                        }
                    }
                }
                let last = {
                    let mut state = fake.lock();
                    state.ticker_reads += 1;
                    state.ticker_last.clone()
                };
                let (status, body) = match last {
                    Some(last) => (
                        "200 OK",
                        json!({ "last": last, "bid": "1.00", "ask": "999999.00", "timestamp": "1793448000" }).to_string(),
                    ),
                    None => ("500 Internal Server Error", "{}".to_owned()),
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    }
}
