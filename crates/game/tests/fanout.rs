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

            #[tokio::test]
            async fn a_price_published_just_before_closing_reaches_every_subscription_before_it_ends() {
                suite::a_price_published_just_before_closing_reaches_every_subscription_before_it_ends($new).await;
            }

            #[tokio::test]
            async fn a_players_event_reaches_every_stream_of_that_player_and_no_other() {
                suite::a_players_event_reaches_every_stream_of_that_player_and_no_other($new).await;
            }

            #[tokio::test]
            async fn a_player_gets_every_event_in_the_order_it_was_published() {
                suite::a_player_gets_every_event_in_the_order_it_was_published($new).await;
            }

            #[tokio::test]
            async fn a_stream_whose_queue_of_player_events_is_full_is_closed() {
                suite::a_stream_whose_queue_of_player_events_is_full_is_closed($new).await;
            }
        }
    };
}

shared_suite!(broadcast, Broadcast::default());
shared_suite!(recording, Recording::new());
