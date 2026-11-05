//! Scoped native application storage.
pub mod store;

#[cfg(test)]
mod store_tests;

#[cfg(test)]
mod sqlite_tests;

mod sqlite;

#[cfg(test)]
mod postgres_tests;

mod postgres;
