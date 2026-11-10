mod config_api;
mod lifecycle_api;

pub use lifecycle_api::AppRoutingError;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod config_api_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod lifecycle_api_tests;
