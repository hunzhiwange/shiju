//! Native model catalog logic. Storage and native publication are owned by callers.

#[cfg(target_os = "macos")]
pub use app_model_discovery::discovery_agy;
#[cfg(not(target_arch = "wasm32"))]
pub use app_models::discovery_http;
#[cfg(target_os = "macos")]
pub use app_model_discovery::discovery_opencode;
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

pub use app_model_catalog::{
    capabilities, codex_projection, codex_template, identity, pi_thinking, preset_catalog,
    preset_claude, preset_codex, preset_gemini_grok, preset_hermes_mcode, preset_kimi_dsh,
    preset_omp, preset_opencode_openclaw, preset_pi, preset_search, user_agent_presets, validate,
};
