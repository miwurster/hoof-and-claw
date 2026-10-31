//! Every `Fanout` adapter passes the shared suite.

use game::fanout::suite;
use game::{Broadcast, Recording};

macro_rules! shared_suite {
    ($adapter:ident, $new:expr) => {
        mod $adapter {
            use super::*;

            #[tokio::test]
            async fn every_subscriber_receives_a_published_price() {
                suite::every_subscriber_receives_a_published_price($new).await;
            }

            #[tokio::test]
            async fn a_subscriber_that_does_not_read_gets_only_the_newest_price() {
                suite::a_subscriber_that_does_not_read_gets_only_the_newest_price($new).await;
            }

            #[tokio::test]
            async fn a_subscriber_gets_nothing_published_before_it_subscribed() {
                suite::a_subscriber_gets_nothing_published_before_it_subscribed($new).await;
            }

            #[tokio::test]
            async fn closing_ends_every_subscription() {
                suite::closing_ends_every_subscription($new).await;
            }
        }
    };
}

shared_suite!(broadcast, Broadcast::new());
shared_suite!(recording, Recording::new());
