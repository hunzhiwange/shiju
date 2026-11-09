//! Native usage queries and their account, credential, and capability selection.

#[cfg(not(target_arch = "wasm32"))]
pub mod balances;
pub mod binding;
#[cfg(not(target_arch = "wasm32"))]
pub mod coding;
pub mod detect;
#[cfg(target_os = "macos")]
pub mod subscriptions;

#[cfg(test)]
mod binding_tests;
#[cfg(test)]
mod detect_tests;
