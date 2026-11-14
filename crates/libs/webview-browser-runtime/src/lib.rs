//! Ordinary browser lifecycle and event-loop orchestration.
#![cfg(not(any(target_arch = "wasm32", coverage)))]

mod launch;
mod control_pipe;
mod window_events;
mod runtime;

pub use runtime::run;

#[cfg(test)]
mod runtime_tests;
