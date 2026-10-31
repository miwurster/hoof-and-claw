//! The per-IP key of the rate limits: an entry of a header the proxy appends to, counted from the right, or the peer
//! address without that header.

use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroUsize;

use axum::extract::ConnectInfo;
use axum::http::{HeaderMap, HeaderName, Request};
use tower_governor::GovernorError;
use tower_governor::key_extractor::KeyExtractor;

/// Which address a request comes from, as the rate limits count it.
///
/// `SmartIpKeyExtractor` is not used: it takes the leftmost entry of `X-Forwarded-For`, which a client can forge when
/// the proxy appends to a header the client sent.
#[derive(Clone, Debug)]
pub struct ClientIp {
    header: HeaderName,
    from_right: NonZeroUsize,
}

impl Default for ClientIp {
    /// The rightmost entry of `X-Forwarded-For`, the one Render's proxy appended.
    fn default() -> Self {
        Self::new(
            HeaderName::from_static("x-forwarded-for"),
            NonZeroUsize::MIN,
        )
    }
}

impl ClientIp {
    /// Takes the entry `from_right` places from the right of `header`: 1 is the rightmost.
    #[must_use]
    pub const fn new(header: HeaderName, from_right: NonZeroUsize) -> Self {
        Self { header, from_right }
    }

    /// The address of a request with these headers from this peer; `None` when neither tells one.
    #[must_use]
    pub fn of(&self, headers: &HeaderMap, peer: Option<SocketAddr>) -> Option<IpAddr> {
        self.forwarded(headers)
            .or_else(|| peer.map(|peer| peer.ip()))
    }

    fn forwarded(&self, headers: &HeaderMap) -> Option<IpAddr> {
        // Several header lines read as one list, in order (RFC 9110, section 5.3).
        let entries: Vec<&str> = headers
            .get_all(&self.header)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(','))
            .map(str::trim)
            .collect();
        let index = entries.len().checked_sub(self.from_right.get())?;
        entries.get(index)?.parse().ok()
    }
}

impl KeyExtractor for ClientIp {
    type Key = IpAddr;

    fn extract<T>(&self, request: &Request<T>) -> Result<IpAddr, GovernorError> {
        let peer = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(peer)| *peer);
        self.of(request.headers(), peer)
            .ok_or(GovernorError::UnableToExtractKey)
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    const PEER: &str = "192.0.2.1:41000";

    fn headers(lines: &[&'static str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for line in lines {
            headers.append("x-forwarded-for", HeaderValue::from_static(line));
        }
        headers
    }

    fn key(client_ip: &ClientIp, lines: &[&'static str]) -> Option<IpAddr> {
        client_ip.of(&headers(lines), Some(PEER.parse().unwrap()))
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap()
    }

    #[test]
    fn the_key_is_the_rightmost_x_forwarded_for_entry() {
        assert_eq!(
            key(&ClientIp::default(), &["203.0.113.7"]),
            Some(ip("203.0.113.7"))
        );
        assert_eq!(
            key(&ClientIp::default(), &["198.51.100.4, 203.0.113.7"]),
            Some(ip("203.0.113.7"))
        );
    }

    #[test]
    fn a_forged_leftmost_entry_does_not_change_the_key() {
        let honest = key(&ClientIp::default(), &["203.0.113.7"]);

        for forged in [
            &["10.0.0.1, 203.0.113.7"][..],
            &["1.2.3.4, 5.6.7.8, 203.0.113.7"],
            &["not an address, 203.0.113.7"],
            &["10.0.0.1", "203.0.113.7"],
        ] {
            assert_eq!(key(&ClientIp::default(), forged), honest, "{forged:?}");
        }
    }

    #[test]
    fn without_the_header_the_key_is_the_peer_address() {
        assert_eq!(key(&ClientIp::default(), &[]), Some(ip("192.0.2.1")));
    }

    #[test]
    fn an_entry_that_is_no_address_falls_back_to_the_peer_address() {
        assert_eq!(
            key(&ClientIp::default(), &["unknown"]),
            Some(ip("192.0.2.1"))
        );
    }

    #[test]
    fn an_ipv6_entry_is_a_key_of_its_own() {
        assert_eq!(
            key(&ClientIp::default(), &["203.0.113.7, 2001:db8::1"]),
            Some(ip("2001:db8::1"))
        );
    }

    #[test]
    fn the_header_and_the_position_are_configuration() {
        let second_from_right = ClientIp::new(
            HeaderName::from_static("x-forwarded-for"),
            NonZeroUsize::new(2).unwrap(),
        );
        let other_header =
            ClientIp::new(HeaderName::from_static("true-client-ip"), NonZeroUsize::MIN);
        let mut true_client_ip = HeaderMap::new();
        true_client_ip.insert("true-client-ip", HeaderValue::from_static("203.0.113.9"));

        assert_eq!(
            key(&second_from_right, &["10.0.0.1, 203.0.113.7, 198.51.100.4"]),
            Some(ip("203.0.113.7"))
        );
        assert_eq!(
            key(&second_from_right, &["203.0.113.7"]),
            Some(ip("192.0.2.1")),
            "too few entries fall back to the peer address"
        );
        assert_eq!(
            other_header.of(&true_client_ip, Some(PEER.parse().unwrap())),
            Some(ip("203.0.113.9"))
        );
    }

    #[test]
    fn without_header_or_peer_there_is_no_key() {
        assert_eq!(ClientIp::default().of(&HeaderMap::new(), None), None);
    }
}
