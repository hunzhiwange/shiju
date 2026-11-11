//! Credential-free projection of one saved provider for queue eligibility.
use serde_json::Value;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        AppProviderSummary,
        claude::{ClaudeConfig, ClaudeProfile},
        codex_catalog::CodexProfile,
        gemini::{GeminiConfig, GeminiProfile},
        grok::{GrokConfig, GrokProfile},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderAuth {
    ThirdParty,
    OfficialAccount,
    Unknown,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProviderSnapshot {
    pub provider: AppProviderSummary,
    pub auth: ProviderAuth,
}

/// The storage owner supplies the application, ID and record from the same scoped read.
pub fn snapshot(
    app_id: AppId,
    id: &str,
    row: &Value,
    managed_codex_login: bool,
) -> Result<ProviderSnapshot, String> {
    super::validate_identity(id)?;
    let (name, mut auth) = classify(app_id, row)?;
    if app_id == AppId::Codex && managed_codex_login {
        auth = ProviderAuth::OfficialAccount;
    } else if auth == ProviderAuth::ThirdParty
        && (row.pointer("/meta/authBinding").is_some()
            || row.pointer("/meta/claudeAccountId").is_some()
            || row.pointer("/meta/codexAccountId").is_some()
            || row.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
            || row["category"].as_str().is_some_and(|value| value.eq_ignore_ascii_case("official")))
    {
        // Imported bindings and labels alone cannot establish a credential source.
        auth = ProviderAuth::Unknown;
    }
    Ok(ProviderSnapshot {
        provider: AppProviderSummary { id: id.into(), app_id, name, configured: true },
        auth,
    })
}

fn classify(app: AppId, row: &Value) -> Result<(String, ProviderAuth), String> {
    use ProviderAuth::{OfficialAccount, ThirdParty, Unknown};
    const INVALID: &str = "invalid saved application provider profile";
    let native = match app {
        AppId::Claude => row.get("claude").map(|value| {
            let profile: ClaudeProfile =
                serde_json::from_value(value.clone()).map_err(|_| INVALID)?;
            super::claude::settings(&profile).map_err(|_| INVALID)?;
            let auth = match profile.config {
                ClaudeConfig::Official { .. } => OfficialAccount,
                ClaudeConfig::ThirdParty { api_key, .. } if has_saved_key(&api_key) => ThirdParty,
                _ => Unknown,
            };
            Ok((profile.name, auth))
        }),
        AppId::Codex => row.get("codex").map(|value| {
            let profile: CodexProfile =
                serde_json::from_value(value.clone()).map_err(|_| INVALID)?;
            let auth = codex_auth(row, &profile);
            Ok((profile.name, auth))
        }),
        AppId::Gemini => row.get("gemini").map(|value| {
            let profile: GeminiProfile =
                serde_json::from_value(value.clone()).map_err(|_| INVALID)?;
            super::gemini::settings(&profile).map_err(|_| INVALID)?;
            let auth = match profile.config {
                GeminiConfig::Official {} => OfficialAccount,
                GeminiConfig::ThirdParty { api_key, .. } if has_saved_key(&api_key) => ThirdParty,
                GeminiConfig::ThirdParty { .. } => Unknown,
            };
            Ok((profile.name, auth))
        }),
        AppId::GrokBuild => row.get("grok").map(|value| {
            let profile: GrokProfile =
                serde_json::from_value(value.clone()).map_err(|_| INVALID)?;
            super::grok::settings(&profile).map_err(|_| INVALID)?;
            let auth = match profile.config {
                GrokConfig::Official {} => OfficialAccount,
                GrokConfig::ThirdParty { toml } => grok_auth(&toml),
            };
            Ok((profile.name, auth))
        }),
        _ => return Err("application does not support routing".into()),
    };
    if let Some(native) = native {
        return native.map_err(str::to_owned);
    }
    if let Ok(profile) = super::update::profile(row)
        && super::validate_profile(app, &profile).is_ok()
        && row.get("settingsConfig").is_none()
        && has_saved_key(&profile.api_key)
    {
        return Ok((profile.name, ThirdParty));
    }
    let name = row["name"].as_str().ok_or(INVALID)?;
    Ok((name.into(), Unknown))
}

#[cfg(target_os = "macos")]
fn codex_auth(row: &Value, profile: &CodexProfile) -> ProviderAuth {
    use super::projection::codex_route::{CodexProjection, Route, RouteAuth, RowInput};

    if super::codex_catalog::form::official_binding_model(row).is_ok() {
        return ProviderAuth::OfficialAccount;
    }
    let settings = super::codex_catalog::settings(profile);
    if row.get("settingsConfig") != Some(&settings)
        || super::codex::project("", &settings, None, false).is_err()
    {
        return ProviderAuth::Unknown;
    }
    let input = RowInput { settings: &settings, official: false, proxy_injected_oauth: false };
    if let Ok(CodexProjection { route: Route::Custom { table, auth: RouteAuth::Bearer }, .. }) =
        CodexProjection::of(&input)
        && table
            .get("base_url")
            .and_then(toml_edit::Item::as_str)
            .is_some_and(|url| super::validation::endpoint(url).is_ok())
        && table
            .get("experimental_bearer_token")
            .and_then(toml_edit::Item::as_str)
            .is_some_and(has_saved_key)
    {
        return ProviderAuth::ThirdParty;
    }
    ProviderAuth::Unknown
}

#[cfg(not(target_os = "macos"))]
fn codex_auth(_: &Value, _: &CodexProfile) -> ProviderAuth {
    ProviderAuth::Unknown
}

fn grok_auth(config: &str) -> ProviderAuth {
    let Ok(doc) = config.parse::<toml_edit::DocumentMut>() else {
        return ProviderAuth::Unknown;
    };
    let selected = super::projection::grok_selected(&doc).and_then(|id| doc.get("model")?.get(&id));
    let Some(table) = selected else { return ProviderAuth::Unknown };
    let endpoint = table.get("base_url").and_then(toml_edit::Item::as_str);
    let key = table.get("api_key").and_then(toml_edit::Item::as_str);
    if endpoint.is_some_and(|url| super::validation::endpoint(url).is_ok())
        && key.is_some_and(has_saved_key)
    {
        ProviderAuth::ThirdParty
    } else {
        ProviderAuth::Unknown
    }
}

fn has_saved_key(key: &str) -> bool {
    let key = key.trim();
    !matches!(key, "" | "PROXY_MANAGED" | "CC_SWITCH_PENDING_KEY" | "[redacted]" | "<redacted>")
        && !key.chars().all(|c| matches!(c, '*' | '•'))
}
