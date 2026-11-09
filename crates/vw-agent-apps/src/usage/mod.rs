//! Native usage queries and their account, credential, and capability selection.

#[cfg(not(target_arch = "wasm32"))]
pub mod balances;
pub mod binding;
pub mod cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod coding;
pub mod detect;
pub mod presentation;
pub mod scripts;
#[cfg(target_os = "macos")]
pub mod subscriptions;

#[cfg(test)]
mod binding_tests;
#[cfg(all(test, target_os = "macos"))]
mod cache_tests;
#[cfg(test)]
mod detect_tests;
