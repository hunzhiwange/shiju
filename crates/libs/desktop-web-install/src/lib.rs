//! Local Web package installation and loopback service lifecycle.
#![cfg(not(target_arch = "wasm32"))]

mod web_install;

pub use web_install::{Event, Snapshot, State, update};
