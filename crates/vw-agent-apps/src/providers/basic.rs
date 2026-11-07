//! Basic fields preserve all opaque application configuration. No native activation.
use serde_json::Value;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        basic::ProviderBasic, create::ProviderSecretWrite, presentation::AppProviderPresentation,
        update::AppProviderUpdate,
    },
};
pub fn read(stored: &Value) -> Result<ProviderBasic, String> {
    let p = super::presentation::read(stored)?;
    Ok(ProviderBasic {
        name: stored["name"].as_str().ok_or("invalid stored application provider profile")?.into(),
        category: p.category,
        notes: p.notes,
        website_url: stored
            .get("websiteUrl")
            .filter(|v| !v.is_null())
            .map(|v| {
                v.as_str().map(str::to_owned).ok_or("invalid stored application provider profile")
            })
            .transpose()?,
    })
}
pub fn apply(
    app: AppId,
    stored: &Value,
    fields: &ProviderBasic,
    secret: &ProviderSecretWrite,
) -> Result<Value, String> {
    if fields.name.trim().is_empty()
        || fields.name.len() > 128
        || fields.name.chars().any(char::is_control)
    {
        return Err("invalid provider name".into());
    }
    if let Some(site) = &fields.website_url {
        let url = reqwest::Url::parse(site).map_err(|_| "invalid provider website")?;
        if site.len() > 2048
            || !matches!(url.scheme(), "https" | "http")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || site.chars().any(char::is_control)
        {
            return Err("invalid provider website".into());
        }
    }
    let p = super::presentation::read(stored)?;
    let mut value = super::presentation::update(
        stored,
        &AppProviderPresentation {
            sort_index: p.sort_index,
            category: fields.category.clone(),
            notes: fields.notes.clone(),
        },
    )?;
    if let ProviderSecretWrite::Replace { value: key } = secret {
        let marker = key.trim();
        if marker.eq_ignore_ascii_case("[redacted]") || marker.eq_ignore_ascii_case("<redacted>") {
            return Err("credential placeholder cannot replace a secret; use keep".into());
        }
    }
    match secret {
        ProviderSecretWrite::Keep => {}
        // Native credential variants are edited by their own application form/adapter.
        _ => {
            value = super::update::update(
                app,
                "basic-edit",
                &value,
                &AppProviderUpdate { api_key: secret.clone(), ..Default::default() },
            )?;
        }
    }
    value["name"] = fields.name.clone().into();
    // Native summaries read their typed profile's name, while the catalog also stores a label.
    let profile_key = match app {
        AppId::Claude => "claude",
        AppId::ClaudeDesktop => "claude_desktop",
        AppId::Codex => "codex",
        AppId::Gemini => "gemini",
        AppId::GrokBuild => "grok",
        AppId::OpenCode => "opencode",
        AppId::OpenClaw => "openclaw",
        AppId::Hermes => "hermes",
        AppId::Pi => "pi",
        AppId::Omp => "omp",
        AppId::Mcode => "mcode",
        AppId::Kimi => "kimi",
        AppId::Dsh => "dsh",
        AppId::Agy => "agy",
    };
    if let Some(profile) = value.get_mut(profile_key) {
        let object =
            profile.as_object_mut().ok_or("invalid stored application provider profile")?;
        object.insert("name".into(), fields.name.clone().into());
    }
    if let Some(site) = &fields.website_url {
        value["websiteUrl"] = site.clone().into();
    } else {
        value
            .as_object_mut()
            .ok_or("invalid stored application provider profile")?
            .remove("websiteUrl");
    }
    Ok(value)
}
