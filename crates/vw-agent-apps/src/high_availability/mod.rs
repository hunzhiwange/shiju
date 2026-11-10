//! Domain rules for application routing availability.

pub mod queue;

#[cfg(test)]
mod queue_tests;

#[cfg(not(target_arch = "wasm32"))]
pub mod retry;
