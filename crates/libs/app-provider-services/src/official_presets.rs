//! Tenant defaults for the applications that support native official login.
use super::*;
use app_settings_store::settings_store::GatewaySetting;
#[cfg(target_os = "macos")]
use serde_json::Value;
use serde_json::json;
use vw_agent_apps::providers;
use vw_api_app_provider_base::claude::{ClaudeConfig, ClaudeProfile};
use vw_api_app_provider_extended::grok::{GrokConfig, GrokProfile};
use vw_api_app_provider_profiles::{
    claude_desktop::{ClaudeDesktopConfig, ClaudeDesktopProfile},
    codex_catalog::CodexProfile,
    gemini::{GeminiConfig, GeminiProfile},
};

const INITIALIZED: &str = "app_providers.official_presets";

impl AppProviderStore {
    /// Provision defaults once per tenant, including tenants with existing provider records.
    pub fn ensure_official_preset(&self, tenant: &TenantScope, app: AppId) -> Result<(), String> {
        let Some((id, profile_key, name, website)) = preset(app) else {
            return Ok(());
        };
        let storage_error = |_| "application provider storage unavailable".to_string();
        if self.settings.get(tenant, INITIALIZED, id).map_err(storage_error)?.is_some() {
            return Ok(());
        }
        let (profile, settings) = match app {
            AppId::Claude => {
                let profile = ClaudeProfile {
                    name: name.into(),
                    config: ClaudeConfig::Official { account: None },
                    model: None,
                };
                (json!(profile), providers::claude::settings(&profile))
            }
            AppId::ClaudeDesktop => {
                let profile = ClaudeDesktopProfile {
                    name: name.into(),
                    config: ClaudeDesktopConfig::Official {},
                };
                (json!(profile), providers::claude_desktop::settings(&profile))
            }
            AppId::Codex => {
                let profile = CodexProfile {
                    name: name.into(),
                    config: String::new(),
                    api_key: None,
                    preserve_login: true,
                };
                (json!(profile), Ok(providers::codex_catalog::settings(&profile)))
            }
            AppId::Gemini => {
                let profile = GeminiProfile {
                    name: name.into(),
                    config: GeminiConfig::Official {},
                    model: None,
                };
                (json!(profile), providers::gemini::settings(&profile))
            }
            AppId::GrokBuild => {
                let profile = GrokProfile { name: name.into(), config: GrokConfig::Official {} };
                (json!(profile), providers::grok::settings(&profile))
            }
            _ => unreachable!("official preset must have a profile"),
        };
        let mut record = json!({
            "name": name,
            "category": "official",
            "websiteUrl": website,
            "builtinOfficialPreset": true,
            "settingsConfig": settings.map_err(|_| "invalid official provider preset")?,
        });
        record[profile_key] = profile;
        self.settings
            .initialize_default(
                tenant,
                (INITIALIZED, id),
                NAMESPACE,
                &GatewaySetting {
                    config_key: Self::key(app, id)?,
                    payload_json: record.to_string(),
                },
            )
            .map_err(storage_error)
    }

    #[cfg(target_os = "macos")]
    pub(super) fn is_official_preset(
        &self,
        tenant: &TenantScope,
        app: AppId,
        id: &str,
    ) -> Result<bool, String> {
        if !preset(app).is_some_and(|(preset_id, ..)| id == preset_id) {
            return Ok(false);
        }
        let record = self
            .settings
            .get(tenant, NAMESPACE, &Self::key(app, id)?)
            .map_err(|_| "application provider storage unavailable")?;
        let record: Option<Value> = record
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(|_| "invalid stored application provider profile")?;
        Ok(record.is_some_and(|record| record["builtinOfficialPreset"] == true))
    }
}

fn preset(app: AppId) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    Some(match app {
        AppId::Claude => (
            "claude-official",
            "claude",
            "Claude Official",
            "https://www.anthropic.com/claude-code",
        ),
        AppId::ClaudeDesktop => (
            "claude-desktop-official",
            "claude_desktop",
            "Claude Desktop Official",
            "https://claude.ai/download",
        ),
        AppId::Codex => ("codex-official", "codex", "OpenAI Official", "https://chatgpt.com/codex"),
        AppId::Gemini => ("gemini-official", "gemini", "Google Official", "https://ai.google.dev/"),
        AppId::GrokBuild => ("grokbuild-official", "grok", "Grok Official", "https://x.ai/grok"),
        _ => return None,
    })
}
