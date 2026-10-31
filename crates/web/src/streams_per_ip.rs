//! The cap on streams an IP may hold open at once (ADR 0014: the eleventh is refused with `rateLimited`).

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex, PoisonError};

/// How many streams each IP holds open, shared by every stream route.
#[derive(Clone, Debug)]
pub(crate) struct StreamsPerIp {
    cap: usize,
    open: Arc<Mutex<HashMap<IpAddr, usize>>>,
}

/// A place an open stream holds; dropping it, as the stream ends, frees the place.
#[derive(Debug)]
pub(crate) struct Place {
    ip: IpAddr,
    open: Arc<Mutex<HashMap<IpAddr, usize>>>,
}

impl StreamsPerIp {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            cap,
            open: Arc::default(),
        }
    }

    /// Takes a place for a new stream of `ip`, or `None` when the IP already holds the cap.
    pub(crate) fn open(&self, ip: IpAddr) -> Option<Place> {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        let count = open.entry(ip).or_default();
        if *count >= self.cap {
            return None;
        }
        *count += 1;
        Some(Place {
            ip,
            open: Arc::clone(&self.open),
        })
    }
}

impl Drop for Place {
    fn drop(&mut self) {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = open.get_mut(&self.ip) {
            *count -= 1;
            if *count == 0 {
                open.remove(&self.ip);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::StreamsPerIp;

    const ALICE: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
    const BOB: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 2));

    #[test]
    fn an_ip_may_hold_the_cap_and_no_more() {
        let streams = StreamsPerIp::new(2);
        let first = streams.open(ALICE);
        let second = streams.open(ALICE);
        assert!(first.is_some() && second.is_some());
        assert!(streams.open(ALICE).is_none());
    }

    #[test]
    fn every_ip_has_its_own_cap() {
        let streams = StreamsPerIp::new(1);
        let _alice = streams.open(ALICE);
        assert!(streams.open(BOB).is_some());
    }

    #[test]
    fn a_closed_stream_frees_its_place() {
        let streams = StreamsPerIp::new(1);
        let first = streams.open(ALICE);
        drop(first);
        assert!(streams.open(ALICE).is_some());
    }
}
