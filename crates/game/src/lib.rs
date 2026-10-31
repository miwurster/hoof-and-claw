//! The running game: takes prices from the `PriceSource`, keeps the latest price, and publishes it through `Fanout`.

pub mod clock;
pub mod fanout;
mod game;

pub use clock::Clock;
pub use fanout::{Broadcast, Fanout, Recording, Subscription};
pub use game::{Game, Snapshot};
