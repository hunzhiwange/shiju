//! Pure catalog metadata projection; never reads or writes native application configuration.
use vw_api_types::app_providers::presentation::AppProviderPresentation;
pub fn read(stored: &serde_json::Value) -> Result<AppProviderPresentation, String> {
    if !stored.is_object() {
        return Err("invalid stored application provider profile".into());
    }
    serde_json::from_value(serde_json::json!({
        "sortIndex": stored.get("sortIndex"),
        "category": stored.get("category"),
        "notes": stored.get("notes"),
    }))
    .map_err(|_| "invalid stored application provider profile".into())
}
pub fn update(
    stored: &serde_json::Value,
    request: &AppProviderPresentation,
) -> Result<serde_json::Value, String> {
    if let Some(category) = &request.category
        && (category.len() > 128 || category.chars().any(char::is_control))
    {
        return Err("invalid application provider category".into());
    }
    if let Some(notes) = &request.notes
        && (notes.len() > 8192
            || notes.chars().any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t')))
    {
        return Err("invalid application provider notes".into());
    }
    let mut updated = stored.clone();
    let object = updated.as_object_mut().ok_or("invalid stored application provider profile")?;
    let fields =
        serde_json::to_value(request).map_err(|_| "invalid application provider presentation")?;
    for key in ["sortIndex", "category", "notes"] {
        if fields[key].is_null() {
            object.remove(key);
        } else {
            object.insert(key.into(), fields[key].clone());
        }
    }
    Ok(updated)
}
