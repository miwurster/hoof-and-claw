//! The seed: the REST ticker's `last`, read once after each subscribe (ADR 0007).

use std::sync::Arc;

use prices::Price;
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::http::Uri;

use crate::price::{self, InvalidPrice};

/// More than any ticker answer; a longer answer is cut off and fails to parse.
const LONGEST_ANSWER: u64 = 64 * 1024;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SeedError {
    #[error("the ticker URL {0:?} is not an http or https URL")]
    Url(String),
    #[error("reading the ticker failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("the ticker answered {0:?}")]
    Status(String),
    #[error("the ticker answered without a `last` price")]
    NoLast,
    #[error(transparent)]
    InvalidPrice(#[from] InvalidPrice),
}

/// Reads the ticker's `last`, the price of the last trade; never its bid, ask or other values.
pub(crate) async fn read(tls: &Arc<ClientConfig>, url: &str) -> Result<Price, SeedError> {
    let answer = get(tls, url).await?;
    let (head, body) = answer.split_once("\r\n\r\n").ok_or(SeedError::NoLast)?;
    let status = head.lines().next().unwrap_or_default();
    if status.split(' ').nth(1) != Some("200") {
        return Err(SeedError::Status(status.to_owned()));
    }
    let ticker: Value = serde_json::from_str(body).map_err(|_| SeedError::NoLast)?;
    let last = ticker
        .get("last")
        .and_then(Value::as_str)
        .ok_or(SeedError::NoLast)?;
    Ok(price::parse(last)?)
}

/// One HTTP/1.0 GET, so the answer is neither chunked nor kept alive: it ends when the server closes.
async fn get(tls: &Arc<ClientConfig>, url: &str) -> Result<String, SeedError> {
    let invalid = || SeedError::Url(url.to_owned());
    let uri: Uri = url.parse().map_err(|_| invalid())?;
    let https = match uri.scheme_str() {
        Some("https") => true,
        Some("http") => false,
        _ => return Err(invalid()),
    };
    let host = uri.host().ok_or_else(invalid)?;
    let port = uri.port_u16().unwrap_or(if https { 443 } else { 80 });
    let path = uri.path_and_query().map_or("/", |path| path.as_str());
    let request = format!(
        "GET {path} HTTP/1.0\r\nHost: {host}\r\nAccept: application/json\r\nUser-Agent: hoof-and-claw\r\n\r\n"
    );
    let stream = TcpStream::connect((host, port)).await?;
    if https {
        let name = ServerName::try_from(host.to_owned()).map_err(|_| invalid())?;
        let stream = tokio_rustls::TlsConnector::from(Arc::clone(tls))
            .connect(name, stream)
            .await?;
        exchange(stream, &request).await
    } else {
        exchange(stream, &request).await
    }
}

async fn exchange(
    mut stream: impl AsyncRead + AsyncWrite + Unpin,
    request: &str,
) -> Result<String, SeedError> {
    stream.write_all(request.as_bytes()).await?;
    let mut answer = Vec::new();
    let read = (&mut stream)
        .take(LONGEST_ANSWER)
        .read_to_end(&mut answer)
        .await;
    match read {
        Ok(_) => {}
        // Some servers close a TLS connection without saying so first; the answer is complete all the same.
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof && !answer.is_empty() => {}
        Err(error) => return Err(error.into()),
    }
    Ok(String::from_utf8_lossy(&answer).into_owned())
}
