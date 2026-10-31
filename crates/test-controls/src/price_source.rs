use futures_channel::mpsc;
use futures_util::StreamExt as _;
use prices::{Price, PriceSource, Report};

/// A `PriceSource` that reports what its `SourceControls` tell it to.
pub struct ControllableSource {
    reports: mpsc::UnboundedReceiver<Report>,
}

/// Drives a `ControllableSource`: sends prices, loses and restores the connection.
#[derive(Clone)]
pub struct SourceControls {
    reports: mpsc::UnboundedSender<Report>,
}

/// A controllable Price source and the controls that drive it; it starts Price delayed, as a real one does.
#[must_use]
pub fn controllable_source() -> (ControllableSource, SourceControls) {
    let (sender, receiver) = mpsc::unbounded();
    (
        ControllableSource { reports: receiver },
        SourceControls { reports: sender },
    )
}

impl SourceControls {
    /// The Price source sends a price, which the server receives.
    pub fn send(&self, price: Price) {
        // The controllable source stands in for Bitstamp without trades of its own, so its prices carry no trade id.
        self.report(Report::Received(price, None));
    }

    /// Restores the connection to the Price source, or loses it, which makes the price Price delayed.
    pub fn set_connected(&self, connected: bool) {
        self.report(if connected {
            Report::Live
        } else {
            Report::Delayed
        });
    }

    fn report(&self, report: Report) {
        // The game drops the source only when it stops, and then no report matters any more.
        let _ = self.reports.unbounded_send(report);
    }
}

impl PriceSource for ControllableSource {
    async fn next(&mut self) -> Option<Report> {
        self.reports.next().await
    }
}
