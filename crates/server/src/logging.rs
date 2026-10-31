//! Log events through `tracing`, as text or as one JSON object per line.

use std::io::IsTerminal as _;

use anyhow::Context as _;
use tracing_subscriber::EnvFilter;

use crate::configuration::LogFormat;

pub(crate) fn init(format: LogFormat, filter: &str) -> anyhow::Result<()> {
    let filter =
        EnvFilter::try_new(filter).with_context(|| format!("invalid RUST_LOG {filter:?}"))?;
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(std::io::stdout().is_terminal());
    let result = match format {
        LogFormat::Text => builder.try_init(),
        LogFormat::Json => builder.json().try_init(),
    };
    result
        .map_err(|error| anyhow::anyhow!(error))
        .context("starting the log")
}
