//! Sole native saved-model projection owner. No provider-owned catalog persistence.
use vw_api_types::model_catalog::*;

pub mod identity;
pub mod preset_catalog;
pub mod preset_claude;
pub mod preset_search;

#[cfg(test)]
mod identity_tests;
#[cfg(test)]
mod preset_catalog_tests;
#[cfg(test)]
mod preset_claude_tests;
#[cfg(test)]
mod preset_search_tests;

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
