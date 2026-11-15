//! Native model catalog logic. Storage and native publication are owned by callers.

#[cfg(target_os = "macos")]
pub mod discovery_agy;
#[cfg(not(target_arch = "wasm32"))]
pub use app_models::discovery_http;
#[cfg(target_os = "macos")]
pub mod discovery_opencode;
#[cfg(target_os = "macos")]
mod discovery_process;
pub use app_models::discovery_url;
pub use app_models::edits;
pub use app_models::fill_codex;
pub use app_models::fill_hermes;
pub use app_models::fill_kimi;
pub use app_models::fill_openclaw;
pub use app_models::fill_opencode;
#[cfg(not(target_arch = "wasm32"))]
pub use app_models::fill_pi;
#[cfg(not(target_arch = "wasm32"))]
pub use app_models::fill_service;
#[cfg(not(target_arch = "wasm32"))]
pub use app_models::metadata;

#[cfg(test)]
mod codex_template_tests;
#[cfg(test)]
mod defaults_tests;
#[cfg(all(test, target_os = "macos"))]
mod discovery_agy_tests;
#[cfg(all(test, target_os = "macos"))]
mod discovery_opencode_tests;
#[cfg(test)]
mod fill_hermes_tests;
#[cfg(test)]
mod fill_kimi_tests;
#[cfg(test)]
mod fill_opencode_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod fill_pi_tests;
#[cfg(test)]
mod preset_catalog_tests;
#[cfg(test)]
mod preset_omp_tests;

pub use app_model_catalog::{
    capabilities, codex_projection, codex_template, identity, pi_thinking, preset_catalog,
    preset_claude, preset_codex, preset_gemini_grok, preset_hermes_mcode, preset_kimi_dsh,
    preset_omp, preset_opencode_openclaw, preset_pi, preset_search, user_agent_presets, validate,
};
