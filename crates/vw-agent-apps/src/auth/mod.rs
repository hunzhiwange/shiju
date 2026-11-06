//! Official account credentials. Separate owner from application/provider projections.
pub mod credential_file;

pub mod credential_lease;

#[cfg(all(test, unix))]
mod credential_lease_tests;
