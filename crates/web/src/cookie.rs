//! The identity cookie: issued on the first visit, re-issued with a fresh expiry on every later one (ADR 0001), and
//! limited per IP.

use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse as _, Response};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use identity::Token;

use crate::{ClientIp, ErrorCode};

const NAME: &str = "__Host-player";
/// 400 days, the longest Chrome keeps a cookie.
const MAX_AGE_SECONDS: u32 = 400 * 24 * 60 * 60;
const HOUR: Duration = Duration::from_hours(1);

/// Which address a request comes from, and how many new identity cookies each address has left.
#[derive(Clone)]
pub(crate) struct Cookies {
    client_ip: ClientIp,
    issued: Arc<DefaultKeyedRateLimiter<IpAddr>>,
}

/// The browser's token, or why there is none.
enum Identity {
    Sent(Token),
    Issued(Token),
    /// The address has received its share of new cookies for now.
    RateLimited,
    /// No random bytes are available.
    Unavailable,
}

impl Cookies {
    pub(crate) fn new(client_ip: ClientIp, per_hour: NonZeroU32, burst: NonZeroU32) -> Self {
        let quota = Quota::with_period(HOUR / per_hour.get())
            .unwrap_or_else(|| Quota::per_hour(per_hour))
            .allow_burst(burst);
        Self {
            client_ip,
            issued: Arc::new(RateLimiter::keyed(quota)),
        }
    }

    fn identity(&self, headers: &HeaderMap, peer: Option<SocketAddr>) -> Identity {
        if let Some(token) = sent(headers) {
            return Identity::Sent(token);
        }
        let allowed = self
            .client_ip
            .of(headers, peer)
            .is_none_or(|ip| self.issued.check_key(&ip).is_ok());
        if !allowed {
            return Identity::RateLimited;
        }
        issue().map_or(Identity::Unavailable, Identity::Issued)
    }
}

/// Sets the identity cookie on every response, and hands the token to the route.
pub(crate) async fn on_every_response(
    State(cookies): State<Cookies>,
    mut request: Request,
    next: Next,
) -> Response {
    let token = match cookies.identity(request.headers(), peer(&request)) {
        Identity::Sent(token) | Identity::Issued(token) => token,
        Identity::RateLimited => return ErrorCode::RateLimited.into_response(),
        Identity::Unavailable => return ErrorCode::Internal.into_response(),
    };
    request.extensions_mut().insert(token.clone());
    let mut response = next.run(request).await;
    set(&mut response, &token);
    response
}

/// Sets the identity cookie on the page, `index.html`, and on no other file; past the limit, the page goes without.
pub(crate) async fn on_the_page(
    State(cookies): State<Cookies>,
    request: Request,
    next: Next,
) -> Response {
    let headers = request.headers().clone();
    let peer = peer(&request);
    let mut response = next.run(request).await;
    let is_page = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));
    // Only the page counts against the limit, not the files it loads.
    if !is_page {
        return response;
    }
    match cookies.identity(&headers, peer) {
        Identity::Sent(token) | Identity::Issued(token) => set(&mut response, &token),
        Identity::RateLimited => {}
        Identity::Unavailable => return ErrorCode::Internal.into_response(),
    }
    response
}

fn peer(request: &Request) -> Option<SocketAddr> {
    request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(peer)| *peer)
}

fn sent(headers: &HeaderMap) -> Option<Token> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == NAME)
        .and_then(|(_, value)| value.parse().ok())
}

fn issue() -> Option<Token> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).ok()?;
    Some(Token::from_random_bytes(bytes))
}

fn set(response: &mut Response, token: &Token) {
    let cookie = format!(
        "{NAME}={token}; Path=/; Max-Age={MAX_AGE_SECONDS}; HttpOnly; Secure; SameSite=Lax"
    );
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().append(SET_COOKIE, value);
    }
}
