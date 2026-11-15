#![cfg(not(target_arch = "wasm32"))]
//! External history uses the gateway's tenant and trusted execution-host policy.
mod catalog;
pub use catalog::router;

mod scan;

mod messages;

mod stream;

mod content;

mod images;

mod delete;

mod resume;

mod maintenance;

mod compression;

mod markdown;

pub mod usage_scan;
