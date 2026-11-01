//! The socket the server accepts its connections on.

use std::net::SocketAddr;

use tokio::net::{TcpListener, TcpSocket};

/// How many connections may wait to be accepted, enough for every Player opening the game at once.
///
/// The standard library and `tokio` listen with 128; the kernel caps the backlog at `net.core.somaxconn`.
const BACKLOG: u32 = 1024;

/// Listens on every IPv4 address with a backlog of [`BACKLOG`].
pub(crate) fn listen(port: u16) -> std::io::Result<TcpListener> {
    let socket = TcpSocket::new_v4()?;
    socket.set_reuseaddr(true)?;
    socket.bind(SocketAddr::from(([0, 0, 0, 0], port)))?;
    socket.listen(BACKLOG)
}

// Linux drops a connection attempt that overflows the backlog, and the client retries it only after a second.
// macOS keeps a backlog of 1024 too, but it resets an overflowing attempt only in some runs: with a backlog of 128 the
// test fails there in about one run in four. So the test runs on Linux only, where it fails every time.
#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::net::SocketAddr;
    use std::time::Duration;

    use tokio::net::TcpStream;
    use tokio::time::timeout;

    use super::listen;

    /// More connections than the standard library's backlog of 128, such as 200 Players opening the game at once.
    const AT_ONCE: usize = 200;

    /// Well under the second after which a client retries a dropped connection attempt.
    const WITHOUT_A_RETRY: Duration = Duration::from_millis(500);

    #[tokio::test]
    async fn two_hundred_connections_waiting_at_once_are_all_kept() {
        let listener = listen(0).unwrap();
        let address = SocketAddr::from(([127, 0, 0, 1], listener.local_addr().unwrap().port()));

        let attempts: Vec<_> = (0..AT_ONCE)
            .map(|_| tokio::spawn(timeout(WITHOUT_A_RETRY, TcpStream::connect(address))))
            .collect();
        let mut lost = 0;
        for attempt in attempts {
            if !matches!(attempt.await.unwrap(), Ok(Ok(_))) {
                lost += 1;
            }
        }

        assert_eq!(
            lost, 0,
            "every connection should be kept until it is accepted"
        );
    }
}
