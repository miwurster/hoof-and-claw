//! The game server: reads its configuration, opens the database, and serves the game.

mod configuration;
mod logging;

use anyhow::Context as _;
use clap::Parser as _;
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
    let test_controls = Some(test_controls::routes());
    #[cfg(not(feature = "test-controls"))]
    let test_controls = None;

    let router = web::router(web::Config {
        build: configuration.build,
        app_dir: configuration.app_dir,
        test_controls,
    });
    let listener = TcpListener::bind(("0.0.0.0", configuration.port))
        .await
        .with_context(|| format!("binding port {}", configuration.port))?;
    tracing::info!("listening on {}", listener.local_addr()?);

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serving")?;
    database.close().await;
    tracing::info!("stopped");
    Ok(())
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
