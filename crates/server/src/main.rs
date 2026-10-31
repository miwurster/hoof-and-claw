//! The game server: reads its configuration, opens the database, and serves the game.

mod clock;
mod configuration;
mod logging;

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Context as _;
use clap::Parser as _;
use game::{Broadcast, Clock, Fanout, Game, Store};
use prices::PriceSource;
use tokio::net::TcpListener;

use crate::configuration::Configuration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let configuration = Configuration::parse();
    logging::init(configuration.log_format, &configuration.rust_log)?;

    let database = sqlite::Database::open(&configuration.database_path)
        .await
        .with_context(|| format!("opening {}", configuration.database_path.display()))?;

    // The test controls exist only in a build with the Cargo feature `test-controls`.
    #[cfg(feature = "test-controls")]
    let result = {
        let clock = test_controls::ControllableClock::new();
        let (source, controls) = test_controls::controllable_source();
        let game = Game::new(clock.clone(), Broadcast::new(), database.clone());
        let test_controls = test_controls::routes(test_controls::Controls {
            clock,
            source: controls,
            game: game.clone(),
        });
        serve(configuration, game, source, async {}, Some(test_controls)).await
    };
    #[cfg(not(feature = "test-controls"))]
    let result = {
        let (source, bitstamp) = bitstamp::Bitstamp::start(configuration.bitstamp())?;
        let game = Game::new(clock::SystemClock, Broadcast::new(), database.clone());
        serve(configuration, game, source, bitstamp.close(), None).await
    };

    database.close().await;
    result?;
    tracing::info!("stopped");
    Ok(())
}

async fn serve<C: Clock, F: Fanout, S: Store>(
    configuration: Configuration,
    game: Game<C, F, S>,
    source: impl PriceSource,
    close_source: impl Future<Output = ()> + Send + 'static,
    test_controls: Option<axum::Router>,
) -> anyhow::Result<()> {
    let running = tokio::spawn({
        let game = game.clone();
        async move { game.run(source).await }
    });
    let router = web::router(web::Config {
        game: game.clone(),
        build: configuration.build,
        app_dir: configuration.app_dir,
        keepalive: Duration::from_secs(configuration.sse_keepalive_secs),
        client_ip: web::ClientIp::new(
            configuration.client_ip_header,
            configuration.client_ip_from_right,
        ),
        limits: web::Limits {
            guesses_per_minute: configuration.guess_rate_per_minute,
            guess_burst: configuration.guess_rate_burst,
            cookies_per_hour: configuration.cookie_rate_per_hour,
            cookie_burst: configuration.cookie_rate_burst,
        },
        test_controls,
    });
    let listener = TcpListener::bind(("0.0.0.0", configuration.port))
        .await
        .with_context(|| format!("binding port {}", configuration.port))?;
    tracing::info!("listening on {}", listener.local_addr()?);

    // The peer address is the per-IP key when no proxy header names the client.
    let served = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        shutdown_signal().await;
        // Closing the connection to the Price source makes the price Price delayed (ADR 0007).
        close_source.await;
        // Graceful shutdown waits for every open stream, so the streams end first.
        game.end_streams();
    })
    .await
    .context("serving");
    running.abort();
    served
}

async fn shutdown_signal() {
    let interrupt = tokio::signal::ctrl_c();
    let mut terminate =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(terminate) => terminate,
            Err(error) => {
                tracing::error!("cannot listen for SIGTERM: {error}");
                let _ = interrupt.await;
                return;
            }
        };
    tokio::select! {
        _ = interrupt => {}
        _ = terminate.recv() => {}
    }
    tracing::info!("shutting down");
}
