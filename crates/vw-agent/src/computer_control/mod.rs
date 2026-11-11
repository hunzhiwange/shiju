//! Local computer-control authorization, independent of backend readiness.

pub mod policy;

#[cfg(unix)]
pub mod host;

#[cfg(test)]
mod policy_tests;

#[cfg(all(test, unix))]
mod host_tests;
