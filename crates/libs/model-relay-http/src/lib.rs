//! Loopback-only authorized model HTTP service.
mod server;
pub use server::{RelayBackend, RelayHost, RelayRequest, RelayServer};

#[cfg(test)]
mod host_migration_46_tests;

#[cfg(test)]
mod host_migration_47_tests;
