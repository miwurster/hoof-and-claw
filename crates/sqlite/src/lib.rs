//! Every core's store port on one SQLite database file and one write connection.

mod database;
mod guess;
mod identity_link;

pub use database::{Database, Error};
