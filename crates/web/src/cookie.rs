//! The identity cookie: issued on the first visit, re-issued with a fresh expiry on every later one (ADR 0001).

use axum::extract::Request;
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue};
use axum::middleware::Next;
use axum::response::{IntoResponse as _, Response};
use identity::Token;

use crate::ErrorCode;

const NAME: &str = "__Host-player";
/// 400 days, the longest Chrome keeps a cookie.
const MAX_AGE_SECONDS: u32 = 400 * 24 * 60 * 60;

/// Sets the identity cookie on every response.
pub(crate) async fn on_every_response(request: Request, next: Next) -> Response {
    let Some(token) = token(request.headers()) else {
        return ErrorCode::Internal.into_response();
    };
    let mut response = next.run(request).await;
    set(&mut response, &token);
    response
}

/// Sets the identity cookie on the page, `index.html`, and on no other file.
pub(crate) async fn on_the_page(request: Request, next: Next) -> Response {
    let token = token(request.headers());
    let mut response = next.run(request).await;
    let is_page = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));
    match token {
        Some(token) if is_page => set(&mut response, &token),
        None if is_page => return ErrorCode::Internal.into_response(),
        _ => {}
    }
    response
}

/// The browser's token, or a new one when it sent none or one that is no token; `None` when no random bytes are
/// available.
fn token(headers: &HeaderMap) -> Option<Token> {
    sent(headers).or_else(issue)
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
