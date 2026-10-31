//! Every core's store port on one SQLite database file and one write connection.

mod database;

pub use database::{Database, Error};
