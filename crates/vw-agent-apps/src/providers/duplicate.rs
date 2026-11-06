//! Catalog-only copy. The tenant store remains the sole persistence owner.
use super::{update::profile, validate_identity, validate_profile};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderSummary, duplicate::AppProviderDuplicate},
};
/// Copy established API configuration and descriptive metadata only. Unknown fields,
/// account bindings, live membership, ordering and timestamps never inherit authority.
/// The callback must atomically insert if absent; false denotes a collision.
pub fn duplicate(
    app: AppId,
    source_id: &str,
    stored: &serde_json::Value,
    request: &AppProviderDuplicate,
    insert: impl FnOnce(&serde_json::Value) -> Result<bool, String>,
) -> Result<AppProviderSummary, String> {
    validate_identity(source_id)?;
    validate_identity(&request.new_id)?;
    if source_id == request.new_id {
        return Err("application provider already exists".into());
    }
    let mut value = profile(stored)?;
    value.name = request.name.clone();
    validate_profile(app, &value)?;
    let mut copied =
        serde_json::to_value(&value).map_err(|_| "invalid application provider profile")?;
    // No opaque object is copied: future nested credentials must remain excluded.
    for key in ["websiteUrl", "category", "notes", "icon", "iconColor"] {
        if let Some(text) = stored.get(key).and_then(serde_json::Value::as_str) {
            copied[key] = text.into();
        }
    }
    if let Some(notes) =
        stored.get("meta").and_then(|meta| meta.get("notes")).and_then(serde_json::Value::as_str)
    {
        copied["meta"] = serde_json::json!({"notes": notes});
    }
    if !insert(&copied)? {
        return Err("application provider already exists".into());
    }
    Ok(AppProviderSummary {
        id: request.new_id.clone(),
        app_id: app,
        name: value.name,
        configured: true,
    })
}
