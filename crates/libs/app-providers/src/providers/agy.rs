use vw_api_app_provider_profiles::agy::AgyProfile;
pub fn validate(p: &AgyProfile) -> Result<(), String> {
    let invalid = || "invalid Agy profile".to_string();
    if p.name.trim().is_empty() || p.name.len() > 128 || p.name.chars().any(char::is_control) {
        return Err(invalid());
    }
    for (s, max) in p
        .model
        .iter()
        .map(|s| (s, 256))
        .chain(p.api_key.iter().map(|s| (s, 8192)))
        .chain(p.base_url.iter().map(|s| (s, 2048)))
    {
        if s.trim().is_empty() || s.len() > max || s.chars().any(char::is_control) {
            return Err(invalid());
        }
    }
    if p.model.as_ref().is_some_and(|v| v.starts_with('-'))
        || (p.base_url.is_some() && p.api_key.is_none())
    {
        return Err(invalid());
    }
    if p.api_key.as_ref().is_some_and(|s| {
        matches!(s.as_str(), "[redacted]" | "<redacted>") || s.chars().all(|c| c == '*' || c == '•')
    }) {
        return Err(invalid());
    }
    if let Some(url) = &p.base_url {
        super::validation::endpoint(url)?;
    }
    Ok(())
}
/// Native Agy settings contain only a display model. Keys remain process-only.
pub fn import(bytes: &[u8]) -> Result<Option<AgyProfile>, String> {
    super::import::reject_placeholders(bytes).map_err(|_| "invalid Agy native configuration")?;
    let doc = super::live::patch_json::object_value(bytes)
        .map_err(|_| "invalid Agy native configuration")?;
    let model = match doc.get("model") {
        None | Some(serde_json::Value::Null) => return Ok(None),
        Some(serde_json::Value::String(s)) if s.trim().is_empty() => return Ok(None),
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => return Err("invalid Agy native model".into()),
    };
    let p =
        AgyProfile { name: "default".into(), model: Some(model), api_key: None, base_url: None };
    validate(&p)?;
    Ok(Some(p))
}
