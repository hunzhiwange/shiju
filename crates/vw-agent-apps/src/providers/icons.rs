//! Provider consumption of the shared icon library; no second library or storage owner.
use crate::icons::validation::is_valid_custom_icon;
use vw_api_types::{
    agent_apps::CustomIconEntry,
    app_providers::icons::{AppProviderIcon, AppProviderIconChange},
};
/// Return only a validated inline custom image, never an imported path or URL.
pub fn read(stored: &serde_json::Value) -> Result<AppProviderIcon, String> {
    let object = stored.as_object().ok_or("invalid stored application provider profile")?;
    let data = match object.get("icon") {
        None | Some(serde_json::Value::Null) => None,
        Some(serde_json::Value::String(data)) if is_valid_custom_icon(data) => Some(data.clone()),
        Some(_) => return Err("invalid stored application provider profile".into()),
    };
    Ok(AppProviderIcon { data })
}
/// Resolve only within the authenticated tenant library and copy the data. A library
/// rename/delete cannot invalidate this snapshot; no native configuration is touched.
pub fn update(
    stored: &serde_json::Value,
    request: &AppProviderIconChange,
    icons: &[CustomIconEntry],
) -> Result<serde_json::Value, String> {
    let mut updated = stored.clone();
    let object = updated.as_object_mut().ok_or("invalid stored application provider profile")?;
    match request {
        AppProviderIconChange::Select { icon_id } => {
            super::validate_identity(icon_id)?;
            let mut matches = icons.iter().filter(|icon| icon.id == *icon_id);
            let icon = matches.next().ok_or("custom icon not found")?;
            if matches.next().is_some() || !is_valid_custom_icon(&icon.data) {
                return Err("invalid custom icon reference".into());
            }
            object.insert("icon".into(), serde_json::Value::String(icon.data.clone()));
        }
        AppProviderIconChange::Clear => {
            object.remove("icon");
        }
    }
    object.remove("iconColor");
    Ok(updated)
}
