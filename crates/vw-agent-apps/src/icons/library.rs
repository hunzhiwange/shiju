//! Pure icon-library mutations; callers own persistence, IDs and timestamps.

use super::{name::normalize_icon_name, validation::is_valid_custom_icon};
use vw_api_types::agent_apps::{AddCustomIconResult, AgentAppsErrorCode, CustomIconEntry};

/// Validate and add an icon, returning a replacement collection only for new images.
/// Duplicate data preserves the original entry, including its name, ID and timestamp.
/// The caller supplies the new entry's ID and Unix millisecond timestamp.
pub fn add_icon(
    icons: &[CustomIconEntry],
    name: &str,
    data: &str,
    id: &str,
    created_at: i64,
) -> Result<(AddCustomIconResult, Option<Vec<CustomIconEntry>>), AgentAppsErrorCode> {
    let name = normalize_icon_name(name)?;
    if !is_valid_custom_icon(data) {
        return Err(AgentAppsErrorCode::InvalidInput);
    }
    if let Some(existing) = icons.iter().find(|icon| icon.data == data) {
        return Ok((AddCustomIconResult { icon: existing.clone(), existed: true }, None));
    }
    if icons.len() >= 100 {
        return Err(AgentAppsErrorCode::Conflict);
    }
    let icon = CustomIconEntry { id: id.to_owned(), name, data: data.to_owned(), created_at };
    let mut updated = Vec::with_capacity(icons.len() + 1);
    updated.push(icon.clone());
    updated.extend_from_slice(icons);
    Ok((AddCustomIconResult { icon, existed: false }, Some(updated)))
}
