//! Trusted local computer-control policy, IPC and application observations.

pub mod policy;

#[cfg(unix)]
pub mod host;

#[cfg(test)]
mod policy_tests;

#[cfg(all(test, unix))]
mod host_tests;

pub mod native;
