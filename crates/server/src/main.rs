//! The game server: reads its configuration, opens the database, and serves the game.

mod clock;
mod configuration;
mod listener;
mod logging;

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Context as _;
use clap::Parser as _;
use game::{Broadcast, Clock, Fanout, Game, Store};
use prices::PriceSource;
use tokio::sync::oneshot;
use tokio::time::Instant;

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
        let (store, store_controls) = test_controls::controllable_store(database.clone());
        let game = Game::new(
            clock.clone(),
            Broadcast::new(configuration.player_event_queue),
            store,
        );
        let close_source = {
            let controls = controls.clone();
            async move { controls.set_connected(false) }
        };
        let test_controls = test_controls::routes(test_controls::Controls {
            clock,
            source: controls,
            store: store_controls,
            game: game.clone(),
        });
        serve(
            configuration,
            game,
            source,
            close_source,
            Some(test_controls),
        )
        .await
    };
    #[cfg(not(feature = "test-controls"))]
    let result = {
        let (source, bitstamp) = bitstamp::Bitstamp::start(configuration.bitstamp())?;
        let game = Game::new(
            clock::SystemClock,
            Broadcast::new(configuration.player_event_queue),
            database.clone(),
        );
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
    // A Guess placed before a restart resolves on the first changed price received after it.
    let waiting = game
        .rebuild_due_guesses()
        .await
        .context("reading the Open guesses")?;
    tracing::info!("{waiting} open guesses wait for resolution");
    let running = tokio::spawn({
        let game = game.clone();
        async move { game.run(source).await }
    });
    let drain = Duration::from_secs(configuration.shutdown_drain_secs);
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
            streams_per_ip: configuration.max_streams_per_ip,
        },
        test_controls,
    });
    let listener = listener::listen(configuration.port)
        .with_context(|| format!("binding port {}", configuration.port))?;
    tracing::info!("listening on {}", listener.local_addr()?);

    // The drain's deadline, set when the signal arrives.
    let (deadline_sender, deadline) = oneshot::channel();
    // The peer address is the per-IP key when no proxy header names the client.
    let served = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown({
        let game = game.clone();
        async move {
            shutdown_signal().await;
            let _ = deadline_sender.send(Instant::now() + drain);
            // Closing the connection to the Price source makes the price Price delayed (ADR 0007); the game publishes
            // that and refuses every Guess from now on with `priceDelayed`.
            close_source.await;
            game.shut_down();
            // Graceful shutdown waits for every open stream, so the streams end first.
            game.end_streams();
        }
    })
    .into_future();
    let drained = async {
        served.await.context("serving")?;
        // The game stops taking prices once a running save of resolutions is done.
        if let Err(error) = running.await {
            tracing::error!("the game stopped with an error: {error}");
        }
        Ok(())
    };
    let timed_out = async {
        match deadline.await {
            Ok(deadline) => tokio::time::sleep_until(deadline).await,
            Err(_) => std::future::pending().await,
        }
    };
    tokio::select! {
        served = drained => served,
        () = timed_out => {
            tracing::warn!("shutting down without waiting any longer, after {} s", drain.as_secs());
            Ok(())
        }
    }
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
