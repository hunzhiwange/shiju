//! Saved generation target parsing shared by routing and diagnostics.
//! Callers authorize tenant, execution host, application and provider before reading credentials.
use crate::model_relay::generation::GenerationProtocol;
use serde_json::Value;
use vw_agent_apps::providers;
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{
        ProviderProtocol,
        claude::{ClaudeConfig, ClaudeProfile},
        gemini::{GeminiConfig, GeminiProfile},
    },
    id::ProviderId,
};

#[derive(Clone, Copy)]
pub(crate) struct UnsupportedSavedTarget;

pub(crate) struct SavedProfile {
    pub(crate) endpoint: String,
    pub(crate) model: Option<String>,
    pub(crate) credential: String,
    pub(crate) protocol: GenerationProtocol,
    pub(crate) messages_bearer: bool,
}

impl SavedProfile {
    /// Caller authorizes this saved provider and execution host before resolving secrets.
    pub(crate) fn read_for_host(
        app: AppId,
        _host: &str,
        _id: &ProviderId,
        row: &Value,
    ) -> Result<Self, UnsupportedSavedTarget> {
        let unsupported = UnsupportedSavedTarget;
        if row["category"] == "official"
            || row.pointer("/meta/liveConfigManaged") == Some(&Value::Bool(true))
            || [
                "/meta/authBinding",
                "/meta/githubAccountId",
                "/managedCodexLogin",
                "/managedClaudeLogin",
            ]
            .iter()
            .any(|path| row.pointer(path).is_some_and(|value| !value.is_null()))
        {
            return Err(unsupported);
        }
        let (endpoint, model, credential, default, messages_bearer) = match app {
            AppId::Claude if let Some(native) = row.get("claude") => {
                let profile: ClaudeProfile =
                    serde_json::from_value(native.clone()).map_err(|_| unsupported)?;
                let ClaudeConfig::ThirdParty { base_url, api_key, api_key_header } = profile.config
                else {
                    return Err(unsupported);
                };
                (base_url, profile.model, api_key, GenerationProtocol::Messages, !api_key_header)
            }
            AppId::ClaudeDesktop => {
                // The Desktop mapping owner treats this as a managed identity.
                // A static saved token cannot stand in for its account proof.
                if row.pointer("/meta/providerType").is_some() {
                    return Err(unsupported);
                }
                let env = row.pointer("/settingsConfig/env").ok_or(unsupported)?;
                let endpoint = env["ANTHROPIC_BASE_URL"].as_str().ok_or(unsupported)?.to_owned();
                let (credential, bearer) = if let Some(token) = env["ANTHROPIC_AUTH_TOKEN"].as_str()
                {
                    (token.to_owned(), true)
                } else {
                    (env["ANTHROPIC_API_KEY"].as_str().ok_or(unsupported)?.to_owned(), false)
                };
                (endpoint, None, credential, GenerationProtocol::Messages, bearer)
            }
            AppId::Gemini if let Some(native) = row.get("gemini") => {
                let profile: GeminiProfile =
                    serde_json::from_value(native.clone()).map_err(|_| unsupported)?;
                let GeminiConfig::ThirdParty { base_url, api_key } = profile.config else {
                    return Err(unsupported);
                };
                (base_url, profile.model, api_key, GenerationProtocol::Gemini, false)
            }
            #[cfg(target_os = "macos")]
            AppId::Codex if row.get("codex").is_some() => {
                use vw_agent_apps::providers::codex_catalog;
                let profile =
                    codex_catalog::form::editable_profile(_id.as_ref(), row).ok_or(unsupported)?;
                let identity = codex_catalog::identity(_host, _id.as_ref(), &profile)
                    .map_err(|_| unsupported)?;
                let config: toml::Table = profile.config.parse().map_err(|_| unsupported)?;
                let model = config.get("model").and_then(toml::Value::as_str).map(str::to_owned);
                (
                    identity.endpoint,
                    model,
                    profile.api_key.ok_or(unsupported)?,
                    GenerationProtocol::Responses,
                    false,
                )
            }
            _ => {
                let profile = providers::update::profile(row).map_err(|_| unsupported)?;
                providers::validate_profile(app, &profile).map_err(|_| unsupported)?;
                let protocol = match profile.protocol {
                    ProviderProtocol::Anthropic => GenerationProtocol::Messages,
                    ProviderProtocol::OpenaiResponses => GenerationProtocol::Responses,
                    ProviderProtocol::Gemini => GenerationProtocol::Gemini,
                };
                (profile.base_url, Some(profile.model), profile.api_key, protocol, false)
            }
        };
        let protocol = match row.pointer("/meta/apiFormat") {
            None | Some(Value::Null) => default,
            Some(format) => match format.as_str() {
                Some("anthropic") if matches!(app, AppId::Claude | AppId::ClaudeDesktop) => {
                    GenerationProtocol::Messages
                }
                Some("openai_chat")
                    if matches!(app, AppId::Claude | AppId::Codex | AppId::ClaudeDesktop) =>
                {
                    GenerationProtocol::Chat
                }
                Some("openai_responses") if matches!(app, AppId::Claude | AppId::Codex) => {
                    GenerationProtocol::Responses
                }
                Some("gemini_native") if matches!(app, AppId::Claude | AppId::Gemini) => {
                    GenerationProtocol::Gemini
                }
                _ => return Err(unsupported),
            },
        };
        Ok(Self { endpoint, model, credential, protocol, messages_bearer })
    }
}
