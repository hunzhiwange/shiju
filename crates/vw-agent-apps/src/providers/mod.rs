//! Application-provider validation/projection owner. No model-provider or host credential fallback.
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderWrite, ProviderProtocol},
};

pub fn validate_identity(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err("invalid application provider identity".into());
    }
    Ok(())
}

pub fn validate_profile(app: AppId, profile: &AppProviderWrite) -> Result<(), String> {
    use AppId::*;
    let expected = match app {
        Claude => ProviderProtocol::Anthropic,
        Codex | Omp => ProviderProtocol::OpenaiResponses,
        Gemini | Agy => ProviderProtocol::Gemini,
        ClaudeDesktop => {
            return Err("desktop application has no provider terminal capability".into());
        }
        GrokBuild => return Err("grok provider session projection is not implemented".into()),
        OpenCode => return Err("opencode provider session projection is not implemented".into()),
        OpenClaw => return Err("openclaw provider session projection is not implemented".into()),
        Hermes => return Err("hermes provider session projection is not implemented".into()),
        Pi => return Err("pi provider session projection is not implemented".into()),
        Mcode => return Err("mcode provider session projection is not implemented".into()),
        Kimi => return Err("kimi provider session projection is not implemented".into()),
        Dsh => return Err("dsh provider session projection is not implemented".into()),
    };
    if profile.protocol != expected {
        return Err("provider protocol does not match application".into());
    }
    for (value, max) in [
        (&profile.name, 128),
        (&profile.api_key, 8192),
        (&profile.model, 256),
        (&profile.base_url, 2048),
    ] {
        if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
            return Err("invalid application provider profile".into());
        }
    }
    if profile.model.starts_with('-') {
        return Err("invalid application provider model".into());
    }
    let url = reqwest::Url::parse(&profile.base_url).map_err(|_| "invalid provider endpoint")?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if (url.scheme() != "https" && !(url.scheme() == "http" && local))
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("provider endpoint requires HTTPS or loopback HTTP without credentials, query or fragment".into());
    }
    Ok(())
}

pub mod catalog;
#[cfg(all(test, target_os = "macos"))]
mod catalog_tests;

pub mod create;
#[cfg(test)]
mod create_tests;

pub mod update;
#[cfg(test)]
mod update_tests;

pub mod delete;
#[cfg(all(test, target_os = "macos"))]
mod delete_tests;

pub mod duplicate;
#[cfg(test)]
mod duplicate_tests;

pub mod presentation;
#[cfg(test)]
mod presentation_tests;

pub mod endpoints;
#[cfg(test)]
mod endpoints_tests;

pub mod icons;
#[cfg(test)]
mod icons_tests;
