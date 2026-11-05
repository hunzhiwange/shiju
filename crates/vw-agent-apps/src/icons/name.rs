//! Storage-independent icon name normalization.

use vw_api_types::agent_apps::AgentAppsErrorCode;

/// Trim surrounding whitespace and accept one through 64 Unicode characters.
/// Invalid names return a fixed business code without retaining the input.
pub fn normalize_icon_name(name: &str) -> Result<String, AgentAppsErrorCode> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(AgentAppsErrorCode::InvalidInput);
    }
    Ok(name.to_owned())
}
