//! MCP process event loop and document lifecycle assembly.
#![cfg(not(target_arch = "wasm32"))]

mod document;
mod runtime;
mod stdin_bridge;

pub use runtime::run;
