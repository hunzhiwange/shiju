//! Authorized native directory and provider publication helpers.
#[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
pub mod aggregation_resync;
#[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
pub mod resync_host;
mod error;
pub use error::AggregationApiError;

#[cfg(not(target_arch = "wasm32"))]
pub mod publication_host;

#[cfg(not(target_arch = "wasm32"))]
pub mod aggregation_desktop;

#[cfg(not(target_arch = "wasm32"))]
pub mod aggregation_omo;

#[cfg(not(target_arch = "wasm32"))]
pub mod aggregation_omp;

#[cfg(not(target_arch = "wasm32"))]
pub mod aggregation_pi;

#[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
pub mod aggregation_read_snapshot;
