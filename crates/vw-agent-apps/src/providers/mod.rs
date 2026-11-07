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
    validation::endpoint(&profile.base_url)?;
    if let Some(options) = &profile.request_options {
        validation::validate(app, options)?;
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

pub mod universal;
#[cfg(test)]
mod universal_tests;

pub mod universal_sync;
#[cfg(test)]
mod universal_sync_tests;

pub mod paths;
#[cfg(test)]
mod paths_tests;

pub mod live;

pub mod preview;
#[cfg(all(test, target_os = "macos"))]
mod preview_tests;

mod projection;

pub mod validation;
#[cfg(test)]
mod validation_tests;

pub mod editor_merge;
#[cfg(test)]
mod editor_merge_tests;

pub mod snippets;
#[cfg(test)]
mod snippets_tests;

pub mod claude;
#[cfg(test)]
mod claude_tests;

#[cfg(target_os = "macos")]
pub mod claude_login;
#[cfg(all(test, target_os = "macos"))]
mod claude_login_tests;

pub mod codex;
#[cfg(test)]
mod codex_tests;

#[cfg(target_os = "macos")]
pub mod codex_login;
#[cfg(target_os = "macos")]
mod codex_login_material;
#[cfg(target_os = "macos")]
mod codex_login_policy;
#[cfg(all(test, target_os = "macos"))]
mod codex_login_tests;

#[cfg(target_os = "macos")]
pub mod codex_catalog;
#[cfg(all(test, target_os = "macos"))]
mod codex_catalog_tests;

pub mod gemini;
#[cfg(all(test, target_os = "macos"))]
mod gemini_tests;

pub mod claude_desktop;
pub mod grok;
#[cfg(all(test, target_os = "macos"))]
mod grok_tests;

#[cfg(all(test, target_os = "macos"))]
mod claude_desktop_tests;

pub mod desktop_mapping;
#[cfg(test)]
mod desktop_mapping_tests;

#[cfg(target_os = "macos")]
pub mod hermes;
#[cfg(all(test, target_os = "macos"))]
mod hermes_tests;
#[cfg(target_os = "macos")]
pub mod openclaw;
#[cfg(all(test, target_os = "macos"))]
mod openclaw_tests;
#[cfg(target_os = "macos")]
pub mod opencode;
#[cfg(all(test, target_os = "macos"))]
mod opencode_tests;

#[cfg(target_os = "macos")]
mod opencode_schema;

#[cfg(target_os = "macos")]
pub mod pi;
#[cfg(all(test, target_os = "macos"))]
mod pi_tests;

#[cfg(target_os = "macos")]
pub mod omp;
#[cfg(all(test, target_os = "macos"))]
mod omp_tests;

#[cfg(target_os = "macos")]
pub mod mcode;
#[cfg(all(test, target_os = "macos"))]
mod mcode_tests;

#[cfg(target_os = "macos")]
pub mod kimi;
#[cfg(all(test, target_os = "macos"))]
mod kimi_tests;

#[cfg(target_os = "macos")]
pub mod dsh;
#[cfg(all(test, target_os = "macos"))]
mod dsh_tests;

#[cfg(target_os = "macos")]
pub mod agy;
#[cfg(all(test, target_os = "macos"))]
mod agy_tests;

pub mod import;
#[cfg(test)]
mod import_tests;

#[cfg(target_os = "macos")]
pub mod mode_state;
#[cfg(all(test, target_os = "macos"))]
mod mode_state_tests;

#[cfg(target_os = "macos")]
pub mod mode_apply;
#[cfg(all(test, target_os = "macos"))]
mod mode_apply_tests;

pub mod env_diagnose;
#[cfg(all(test, target_os = "macos"))]
mod env_diagnose_tests;

pub mod env_clean;
#[cfg(all(test, target_os = "macos"))]
mod env_clean_tests;
