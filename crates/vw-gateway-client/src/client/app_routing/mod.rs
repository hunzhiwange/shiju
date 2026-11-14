mod config_api;
mod lifecycle_api;
mod outbound_api;
mod outbound_probe;
mod outbound_scan;

pub use lifecycle_api::AppRoutingError;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod config_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod lifecycle_api_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod outbound_api_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod outbound_probe_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod outbound_scan_tests;

mod request_status;

mod mode_attach;

mod client_host;
