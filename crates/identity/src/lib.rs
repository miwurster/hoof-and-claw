//! Issuing and checking the anonymous token a browser is known by, and the link from a token to its Player (ADR 0001).

pub mod store;
mod token;

pub use store::{InMemory, Store, StoreError};
pub use token::{Token, TokenError, TokenHash};
