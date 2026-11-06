//! Frozen snippet owner: sanitize on every boundary, never write native files.
use serde_json::Value;
use vw_api_types::{agent_apps::AppId, app_providers::snippets::ConfigSnippet};
pub fn normalize(app: AppId, text: &str) -> Result<ConfigSnippet, String> {
    supported(app)?;
    if text.len() > 64 * 1024 {
        return Err("invalid snippet".into());
    }
    if text.trim().is_empty() {
        return Ok(ConfigSnippet { content: None });
    }
    if app == AppId::Codex {
        let value: Value = toml::from_str(text).map_err(|_| "invalid snippet")?;
        return encode(app, value);
    }
    let value: Value = serde_json::from_str(text).map_err(|_| "invalid snippet")?;
    encode(app, value)
}
pub fn extract(app: AppId, settings: &Value) -> Result<ConfigSnippet, String> {
    supported(app)?;
    if !settings.is_object() {
        return Err("invalid snippet".into());
    }
    match app {
        AppId::Codex => {
            normalize(app, settings.get("config").and_then(Value::as_str).ok_or("invalid snippet")?)
        }
        AppId::Gemini => encode(app, settings.get("env").cloned().unwrap_or(serde_json::json!({}))),
        _ => encode(app, settings.clone()),
    }
}
fn supported(app: AppId) -> Result<(), String> {
    match app {
        AppId::Claude | AppId::Codex | AppId::Gemini | AppId::OpenCode | AppId::OpenClaw => Ok(()),
        _ => Err("unsupported snippet application".into()),
    }
}
fn encode(app: AppId, mut value: Value) -> Result<ConfigSnippet, String> {
    if !value.is_object() {
        return Err("invalid snippet".into());
    }
    if serde_json::to_vec(&value).map_err(|_| "invalid snippet")?.len() > 64 * 1024 {
        return Err("invalid snippet".into());
    }
    scrub(&mut value, 0)?;
    let root = value.as_object_mut().ok_or("invalid snippet")?;
    use super::projection::fields;
    match app {
        AppId::Claude => {
            root.retain(|key, _| {
                !fields::claude_floor_top(key)
                    && ![
                        "api_format",
                        "apiFormat",
                        "openrouter_compat_mode",
                        "openrouterCompatMode",
                    ]
                    .contains(&key.as_str())
            });
            if let Some(env) = root.get_mut("env") {
                let env = env.as_object_mut().ok_or("invalid snippet")?;
                env.retain(|key, _| {
                    !fields::claude_floor_env(key)
                        && !["CLAUDE_CODE_MAX_CONTEXT_TOKENS", "CLAUDE_CODE_AUTO_COMPACT_WINDOW"]
                            .contains(&key.as_str())
                });
                if env.is_empty() {
                    root.remove("env");
                }
            }
        }
        AppId::Codex => {
            for key in fields::CODEX_FLOOR_TOP {
                root.remove(*key);
            }
            for path in fields::CODEX_FLOOR_NESTED {
                let &[parent, key] = *path else { continue };
                if let Some(table) = root.get_mut(parent).and_then(Value::as_object_mut) {
                    table.remove(key);
                    if table.is_empty() {
                        root.remove(parent);
                    }
                }
            }
            root.remove("model_providers");
            root.remove("mcp_servers");
            if let Some(mcp) = root.get_mut("mcp").and_then(Value::as_object_mut) {
                mcp.remove("servers");
                if mcp.is_empty() {
                    root.remove("mcp");
                }
            }
            if root.get("web_search").and_then(Value::as_str) == Some("disabled") {
                root.remove("web_search");
            }
        }
        AppId::Gemini => {
            root.retain(|key, value| {
                if fields::gemini_floor_env(key) {
                    return false;
                }
                if let Some(text) = value.as_str() {
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        *value = Value::String(text);
                        return true;
                    }
                }
                false
            });
        }
        AppId::OpenCode => {
            for key in ["options", "settings"] {
                if let Some(options) = root.get_mut(key) {
                    let options = options.as_object_mut().ok_or("invalid snippet")?;
                    options.remove("apiKey");
                    options.remove("baseURL");
                }
            }
        }
        AppId::OpenClaw => {
            root.remove("apiKey");
            root.remove("baseUrl");
        }
        _ => return Err("unsupported snippet application".into()),
    }
    let content = if app == AppId::Codex {
        toml::to_string(&value).map_err(|_| "invalid snippet")?
    } else {
        serde_json::to_string_pretty(&value).map_err(|_| "invalid snippet")?
    };
    if content.len() > 64 * 1024 {
        return Err("invalid snippet".into());
    }
    Ok(ConfigSnippet { content: if content.trim().is_empty() { None } else { Some(content) } })
}

/// Credential patterns from frozen ProviderService::is_sensitive_config_key.
/// Descend into objects/arrays too, so nested plugin credentials cannot become shared secrets.
fn scrub(value: &mut Value, depth: usize) -> Result<(), String> {
    if depth > 64 {
        return Err("invalid snippet".into());
    }
    match value {
        Value::Object(values) => {
            values.retain(|key, _| !sensitive(key));
            for value in values.values_mut() {
                scrub(value, depth + 1)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                scrub(value, depth + 1)?;
            }
        }
        Value::String(text) if text == "[redacted]" || text == "<redacted>" => {
            return Err("invalid snippet".into());
        }
        _ => {}
    }
    Ok(())
}
fn sensitive(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    [
        "APIKEY",
        "API_KEY",
        "ACCESSKEY",
        "ACCESSKEYID",
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "CREDENTIALS",
        "HEADERS",
    ]
    .contains(&upper.as_str())
        || upper.ends_with("TOKEN")
        || [
            "_KEY",
            "_ACCESS_KEY_ID",
            "_KEY_ID",
            "_APIKEY",
            "_ACCESSKEY",
            "_SECRETKEY",
            "_APITOKEN",
            "_TOKEN",
            "_PAT",
            "_PWD",
            "_PASS",
            "_PASSPHRASE",
            "_CREDS",
            "_CUSTOM_HEADERS",
        ]
        .iter()
        .any(|suffix| upper.ends_with(suffix))
        || [
            "SECRET",
            "PASSWORD",
            "PASSWD",
            "CREDENTIAL",
            "PRIVATE_KEY",
            "PRIVATEKEY",
            "BEARER_TOKEN",
            "COOKIE",
            "AUTHORIZATION",
        ]
        .iter()
        .any(|part| upper.contains(part))
}
