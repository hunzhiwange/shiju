//! Model discovery, catalog edits and offline metadata filling.

#[cfg(not(target_arch = "wasm32"))]
pub mod discovery_http;
pub mod discovery_url;
pub mod edits;
pub mod fill_codex;
pub mod fill_hermes;
pub mod fill_kimi;
pub mod fill_openclaw;
pub mod fill_opencode;
#[cfg(not(target_arch = "wasm32"))]
pub mod fill_pi;
#[cfg(not(target_arch = "wasm32"))]
pub mod fill_service;
#[cfg(not(target_arch = "wasm32"))]
pub mod metadata;
#[cfg(not(target_arch = "wasm32"))]
mod vendor_fallback;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod discovery_http_tests;
#[cfg(test)]
mod discovery_url_tests;
#[cfg(test)]
mod edits_tests;
#[cfg(test)]
mod fill_codex_tests;
#[cfg(test)]
mod fill_openclaw_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod fill_service_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod metadata_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod vendor_fallback_tests;

pub use app_model_catalog::{identity, pi_thinking, preset_catalog, preset_kimi_dsh, preset_pi, user_agent_presets};
