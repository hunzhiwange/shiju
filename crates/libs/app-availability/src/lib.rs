//! Domain rules for application routing availability.

pub use app_availability_core::{circuit_breaker, health};

pub mod queue;

#[cfg(test)]
mod queue_tests;

#[cfg(not(target_arch = "wasm32"))]
pub use app_availability_core::retry;

pub mod selection;
