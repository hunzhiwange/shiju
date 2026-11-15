//! Native model catalog logic. Storage and native publication are owned by callers.

#[cfg(target_os = "macos")]
pub mod discovery_agy;
#[cfg(not(target_arch = "wasm32"))]
pub mod discovery_http;
#[cfg(target_os = "macos")]
pub mod discovery_opencode;
#[cfg(target_os = "macos")]
mod discovery_process;
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

#[cfg(test)]
mod codex_template_tests;
#[cfg(test)]
mod defaults_tests;
#[cfg(all(test, target_os = "macos"))]
mod discovery_agy_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod discovery_http_tests;
#[cfg(all(test, target_os = "macos"))]
mod discovery_opencode_tests;
#[cfg(test)]
mod discovery_url_tests;
#[cfg(test)]
mod edits_tests;
#[cfg(test)]
mod fill_codex_tests;
#[cfg(test)]
mod fill_hermes_tests;
#[cfg(test)]
mod fill_kimi_tests;
#[cfg(test)]
mod fill_openclaw_tests;
#[cfg(test)]
mod fill_opencode_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod fill_pi_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod fill_service_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod metadata_tests;
#[cfg(test)]
mod preset_catalog_tests;
#[cfg(test)]
mod preset_omp_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod vendor_fallback_tests;

pub use app_model_catalog::{
    capabilities, codex_projection, codex_template, identity, pi_thinking, preset_catalog,
    preset_claude, preset_codex, preset_gemini_grok, preset_hermes_mcode, preset_kimi_dsh,
    preset_omp, preset_opencode_openclaw, preset_pi, preset_search, user_agent_presets, validate,
};
