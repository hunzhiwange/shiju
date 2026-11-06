//! Desktop catalog mapping owner, independent of proxy runtime/catalog generation.
use serde_json::Value;
use vw_api_types::app_providers::desktop_mapping::*;
pub fn defaults() -> Vec<DesktopDefaultRoute> {
    [
        ("claude-sonnet-5", "ANTHROPIC_DEFAULT_SONNET_MODEL"),
        ("claude-opus-5", "ANTHROPIC_DEFAULT_OPUS_MODEL"),
        ("claude-haiku-4-5", "ANTHROPIC_DEFAULT_HAIKU_MODEL"),
        ("claude-fable-5", "ANTHROPIC_DEFAULT_FABLE_MODEL"),
    ]
    .into_iter()
    .map(|(route_id, env_key)| DesktopDefaultRoute {
        route_id: route_id.into(),
        env_key: env_key.into(),
        supports_1m: true,
    })
    .collect()
}
fn atom(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value == value.trim()
        && value.len() <= max
        && !value.chars().any(char::is_control)
}
fn model_id(value: &str) -> bool {
    atom(value, 256)
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/' | b':'))
}
fn claude_safe(value: &str) -> bool {
    if !model_id(value) {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    let Some(tail) =
        lower.strip_prefix("anthropic/claude-").or_else(|| lower.strip_prefix("claude-"))
    else {
        return false;
    };
    ["sonnet-", "opus-", "haiku-", "fable-"]
        .iter()
        .any(|p| tail.strip_prefix(p).is_some_and(|s| !s.is_empty()))
}
pub fn validate(mapping: &DesktopMapping) -> Result<(), String> {
    if mapping.routes.len() > 200
        || (mapping.mode == DesktopMappingMode::Proxy && mapping.routes.is_empty())
    {
        return Err("invalid Desktop model routes".into());
    }
    for (id, route) in &mapping.routes {
        if !claude_safe(id)
            || !model_id(&route.model)
            || route.label_override.as_ref().is_some_and(|s| !atom(s, 256))
            || (mapping.mode == DesktopMappingMode::Direct && id != &route.model)
        {
            return Err("invalid Desktop model route".into());
        }
    }
    Ok(())
}
/// Convert only stored, compatible rows. Unsupported auth is never promoted to direct.
/// Caller inserts if absent; no files, routing service or current pointers are touched.
pub fn import_row(source: &Value) -> Result<Option<Value>, String> {
    use serde_json::json;
    use vw_api_types::app_providers::{AppProviderWrite, claude_desktop::*};
    let mut source = source.clone();
    if !source.is_object() {
        return Err("invalid provider record".into());
    }
    // The original generic provider API stores a typed profile rather than native settings.
    if source.get("settingsConfig").is_none() {
        let mut fields = source.clone();
        fields.as_object_mut().expect("validated object").retain(|key, _| {
            matches!(
                key.as_str(),
                "name" | "protocol" | "apiKey" | "baseUrl" | "model" | "requestOptions"
            )
        });
        let Ok(profile) = serde_json::from_value::<AppProviderWrite>(fields) else {
            return Ok(None);
        };
        if super::validate_profile(vw_api_types::agent_apps::AppId::Claude, &profile).is_err() {
            return Ok(None);
        }
        source["settingsConfig"] = json!({"env":{"ANTHROPIC_BASE_URL":profile.base_url,"ANTHROPIC_AUTH_TOKEN":profile.api_key,"ANTHROPIC_MODEL":profile.model}});
    }
    if source.pointer("/meta/authBinding").is_some()
        || source.pointer("/meta/providerType").is_some()
        || source.pointer("/meta/liveConfigManaged") == Some(&json!(true))
        || source.pointer("/meta/isFullUrl") == Some(&json!(true))
        || source.pointer("/claude/config/kind").is_some_and(|v| v != "third_party")
    {
        return Ok(None);
    }
    let format = match source.pointer("/meta/apiFormat") {
        None => "anthropic",
        Some(Value::String(value)) => value.as_str(),
        _ => return Err("invalid provider API format".into()),
    };
    if !matches!(format, "" | "anthropic" | "openai_chat" | "openai_responses" | "gemini_native") {
        return Ok(None);
    }
    let Some(env) = source.pointer("/settingsConfig/env").and_then(Value::as_object) else {
        return Ok(None);
    };
    if [
        "CLAUDE_CODE_USE_BEDROCK",
        "CLAUDE_CODE_USE_VERTEX",
        "ANTHROPIC_BEDROCK_BASE_URL",
        "ANTHROPIC_VERTEX_BASE_URL",
    ]
    .iter()
    .any(|key| env.contains_key(*key))
    {
        return Ok(None);
    }
    let get =
        |key: &str| env.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty());
    let Some(base) = get("ANTHROPIC_BASE_URL") else { return Ok(None) };
    let token = [
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_API_KEY",
        "OPENROUTER_API_KEY",
        "OPENAI_API_KEY",
        "GEMINI_API_KEY",
    ]
    .into_iter()
    .find_map(get);
    let Some(token) = token else { return Ok(None) };
    let Some(name) = source.get("name").and_then(Value::as_str) else { return Ok(None) };
    let profile = ClaudeDesktopProfile {
        name: name.into(),
        config: ClaudeDesktopConfig::ThirdParty { base_url: base.into(), auth_token: token.into() },
    };
    if super::claude_desktop::settings(&profile).is_err() {
        return Ok(None);
    }
    // Refuse masked credentials at the import boundary too.
    if matches!(token, "[redacted]" | "<redacted>") || token.chars().all(|c| matches!(c, '*' | '•'))
    {
        return Ok(None);
    }
    let specs = defaults();
    for key in specs.iter().map(|s| s.env_key.as_str()).chain(["ANTHROPIC_MODEL"]) {
        if env.get(key).is_some_and(|v| !v.is_string()) {
            return Err("invalid imported Desktop model".into());
        }
    }
    let safe = specs
        .iter()
        .map(|s| s.env_key.as_str())
        .chain(["ANTHROPIC_MODEL"])
        .all(|k| get(k).is_none_or(claude_safe));
    let direct =
        matches!(format, "" | "anthropic") && get("ANTHROPIC_AUTH_TOKEN").is_some() && safe;
    let mut routes = std::collections::BTreeMap::<String, DesktopModelRoute>::new();
    if !direct {
        for spec in &specs {
            if let Some(raw) = get(&spec.env_key) {
                add_route(
                    &mut routes,
                    &spec.route_id,
                    raw,
                    get(&format!("{}_NAME", spec.env_key)),
                )?;
            }
        }
        if routes.is_empty()
            && let Some(raw) = get("ANTHROPIC_MODEL")
        {
            add_route(&mut routes, &specs[0].route_id, raw, get("ANTHROPIC_MODEL_NAME"))?;
        }
        if routes.is_empty() {
            return Ok(None);
        }
    }
    let mapping = DesktopMapping {
        mode: if direct { DesktopMappingMode::Direct } else { DesktopMappingMode::Proxy },
        routes,
    };
    validate(&mapping)?;
    let obj = source.as_object_mut().expect("validated object");
    for k in ["apiKey", "baseUrl", "protocol", "model", "requestOptions", "claude"] {
        obj.remove(k);
    }
    obj.insert(
        "claude_desktop".into(),
        serde_json::to_value(profile).map_err(|_| "invalid provider profile")?,
    );
    obj.insert("inFailoverQueue".into(), json!(false));
    let meta = obj
        .entry("meta")
        .or_insert(json!({}))
        .as_object_mut()
        .ok_or("invalid provider metadata")?;
    meta.insert("claudeDesktopMode".into(), serde_json::to_value(mapping.mode).unwrap());
    meta.insert("claudeDesktopModelRoutes".into(), serde_json::to_value(mapping.routes).unwrap());
    Ok(Some(source))
}
fn add_route(
    routes: &mut std::collections::BTreeMap<String, DesktopModelRoute>,
    id: &str,
    raw: &str,
    label: Option<&str>,
) -> Result<(), String> {
    let marker = raw.as_bytes();
    let has_marker = marker.len() >= 4 && marker[marker.len() - 4..].eq_ignore_ascii_case(b"[1m]");
    let model = if has_marker { raw[..raw.len() - 4].trim_end() } else { raw };
    if !model_id(model) {
        return Err("invalid imported Desktop model".into());
    }
    let explicit_label = label.is_some();
    let label = label.map(str::to_string).or_else(|| (!claude_safe(model)).then(|| model.into()));
    if let Some(route) = routes.values_mut().find(|r| r.model == model) {
        if route.label_override.is_none()
            || explicit_label
            || route.label_override.as_deref() == Some(model)
        {
            route.label_override = label;
        }
        route.supports_1m = true;
    } else {
        routes.insert(
            id.into(),
            DesktopModelRoute { model: model.into(), label_override: label, supports_1m: true },
        );
    }
    Ok(())
}
