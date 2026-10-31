//! The running game: takes prices from the `PriceSource`, places Guesses through the cores and their store ports, and
//! publishes prices and Player events through `Fanout`.

pub mod clock;
pub mod fanout;
mod game;

pub use clock::Clock;
pub use fanout::{Broadcast, Event, Fanout, PlayerEvent, Recording, Subscription};
pub use game::{Game, Open, PlaceError, Snapshot, Store, Unavailable};
