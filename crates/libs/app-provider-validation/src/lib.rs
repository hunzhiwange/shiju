//! Application-provider validation/projection owner. No model-provider or host credential fallback.
use vw_api_app_provider_base::AppProviderWrite;
use vw_api_app_provider_base::ProviderProtocol;
use vw_api_app_identity::{AppId};

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
    validation::endpoint(&profile.base_url)?;
    if let Some(options) = &profile.request_options {
        validation::validate(app, options)?;
    }
    Ok(())
}

pub mod endpoints;
pub mod request_options;
pub mod validation;
mod projected_endpoint;
pub use projected_endpoint::projected_endpoint;

#[cfg(test)]
mod endpoints_tests;
#[cfg(test)]
mod validation_tests;
