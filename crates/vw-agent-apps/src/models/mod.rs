//! Sole native saved-model projection owner. No provider-owned catalog persistence.
use vw_api_types::model_catalog::*;

pub mod codex_template;
#[cfg(target_os = "macos")]
pub mod discovery_agy;
#[cfg(not(target_arch = "wasm32"))]
pub mod discovery_http;
#[cfg(target_os = "macos")]
pub mod discovery_opencode;
#[cfg(target_os = "macos")]
mod discovery_process;
pub mod discovery_url;
pub mod fill_codex;
pub mod fill_hermes;
pub mod fill_kimi;
pub mod fill_openclaw;
pub mod fill_opencode;
#[cfg(not(target_arch = "wasm32"))]
pub mod fill_pi;
pub mod identity;
#[cfg(not(target_arch = "wasm32"))]
pub mod metadata;
pub mod pi_thinking;
pub mod preset_catalog;
pub mod preset_claude;
pub mod preset_codex;
pub mod preset_gemini_grok;
pub mod preset_hermes_mcode;
pub mod preset_kimi_dsh;
pub mod preset_omp;
pub mod preset_opencode_openclaw;
pub mod preset_pi;
pub mod preset_search;
pub mod user_agent_presets;
#[cfg(not(target_arch = "wasm32"))]
mod vendor_fallback;

#[cfg(test)]
mod codex_template_tests;
#[cfg(all(test, target_os = "macos"))]
mod discovery_agy_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod discovery_http_tests;
#[cfg(all(test, target_os = "macos"))]
mod discovery_opencode_tests;
#[cfg(test)]
mod discovery_url_tests;
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
#[cfg(test)]
mod identity_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod metadata_tests;
#[cfg(test)]
mod pi_thinking_tests;
#[cfg(test)]
mod preset_catalog_tests;
#[cfg(test)]
mod preset_claude_tests;
#[cfg(test)]
mod preset_codex_tests;
#[cfg(test)]
mod preset_gemini_grok_tests;
#[cfg(test)]
mod preset_hermes_mcode_tests;
#[cfg(test)]
mod preset_kimi_dsh_tests;
#[cfg(test)]
mod preset_omp_tests;
#[cfg(test)]
mod preset_opencode_openclaw_tests;
#[cfg(test)]
mod preset_pi_tests;
#[cfg(test)]
mod preset_search_tests;
#[cfg(test)]
mod user_agent_presets_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod vendor_fallback_tests;

pub fn validate(
    identity: &ModelCatalogIdentity,
    catalog: Option<&SavedCodexCatalog>,
) -> Result<(), &'static str> {
    use crate::providers::{validate_identity, validation::endpoint};
    validate_identity(&identity.target_host_id).map_err(|_| "invalid catalog host")?;
    validate_identity(&identity.provider_id).map_err(|_| "invalid catalog provider")?;
    if identity.app_id != vw_api_types::agent_apps::AppId::Codex {
        return Err("unsupported model catalog");
    }
    endpoint(&identity.endpoint).map_err(|_| "invalid catalog endpoint")?;
    if let Some(catalog) = catalog {
        if catalog.models.len() > 256
            || serde_json::to_vec(catalog).map_err(|_| "invalid model catalog")?.len() > 1024 * 1024
        {
            return Err("invalid model catalog");
        }
        let mut seen = std::collections::HashSet::new();
        for row in &catalog.models {
            if row.slug.trim().is_empty()
                || row.slug.len() > 256
                || row.slug.chars().any(char::is_control)
                || !seen.insert(&row.slug)
                || row.base_instructions.len() > 128 * 1024
                || row.display_name.len() > 1024
                || row.description.len() > 8192
                || row.context_window == Some(0)
            {
                return Err("invalid model catalog");
            }
        }
    }
    Ok(())
}
/// Serialize only already-saved native rows, retaining unknown optional metadata as absent.
pub fn codex_projection(
    catalog: Option<&SavedCodexCatalog>,
) -> Result<Option<Vec<u8>>, &'static str> {
    catalog
        .filter(|c| !c.models.is_empty())
        .map(|c| serde_json::to_vec_pretty(c).map_err(|_| "invalid model catalog"))
        .transpose()
}
