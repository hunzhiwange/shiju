//! Mode observations and client-contract decisions share ProviderTransactions state.
//! Values accepted here are executor-owned projections, never HTTP request payloads.
use super::live::commit::{AppState, Contract, Mode};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use vw_api_types::{agent_apps::AppId, app_providers::mode_state::*, id::TenantScope};
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackState {
    pub enabled: bool,
    pub members: Vec<String>,
    pub keys: BTreeMap<String, String>,
}
impl StackState {
    pub fn is_empty(&self) -> bool {
        !self.enabled && self.members.is_empty() && self.keys.is_empty()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeStateError {
    Ownership,
    InvalidConfiguration,
    UnsupportedApplication,
    RouteUnavailable,
}
/// Typed application identity prevents comparing contracts across applications.
/// The contents must be the final client-facing projection, including proxy address,
/// placeholder credential, exclusive values and published models when applicable.
pub enum ClientConfiguration {
    Claude { env: Map<String, Value>, top: Map<String, Value> },
    Gemini { env: Map<String, Value>, config: Value },
    Grok { document: Value },
    ClaudeDesktop { profile: Value },
    Codex { config: Value, auth: Value },
}
impl ClientConfiguration {
    pub fn contract(&self) -> Contract {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        use sha2::{Digest, Sha256};
        let (app, value, exclusive) = match self {
            Self::Claude { env, top } => (
                AppId::Claude,
                json!({"env":env,"top":top}),
                env.iter()
                    .filter(|(k, _)| super::projection::fields::claude_exclusive_env(k))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            ),
            Self::Gemini { env, config } => {
                (AppId::Gemini, json!({"env":env,"config":config}), Map::new())
            }
            Self::Grok { document } => (AppId::GrokBuild, document.clone(), Map::new()),
            Self::ClaudeDesktop { profile } => (AppId::ClaudeDesktop, profile.clone(), Map::new()),
            Self::Codex { config, auth } => {
                let exclusive = config
                    .as_object()
                    .map(|m| {
                        m.iter()
                            .filter(|(k, _)| {
                                super::projection::fields::CODEX_EXCLUSIVE_TOP.contains(&k.as_str())
                            })
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .unwrap_or_default();
                (AppId::Codex, json!({"config":config,"auth":auth}), exclusive)
            }
        };
        // Recursively sort maps even when serde_json's preserve_order feature is enabled.
        fn canonical(v: &Value) -> Value {
            match v {
                Value::Object(m) => Value::Object(
                    m.iter()
                        .map(|(k, v)| (k.clone(), canonical(v)))
                        .collect::<BTreeMap<_, _>>()
                        .into_iter()
                        .collect(),
                ),
                Value::Array(a) => Value::Array(a.iter().map(canonical).collect()),
                _ => v.clone(),
            }
        }
        Contract {
            version: 2,
            key: URL_SAFE_NO_PAD.encode(Sha256::digest(
                serde_json::to_vec(&canonical(
                    &json!({"app":app,"configuration":value,"version":2}),
                ))
                .expect("JSON value serializes"),
            )),
            exclusive,
            claude_login_operation: None,
        }
    }
    /// Only already-saved, known native settings are projected by the executor.
    /// Other clients require their own adapter; do not hash arbitrary row metadata.
    pub fn saved(app: AppId, settings: &Value, official: bool) -> Result<Self, ModeStateError> {
        fn invalid<T>(_: T) -> ModeStateError {
            ModeStateError::InvalidConfiguration
        }
        match app {
            AppId::Claude => {
                if !settings.is_object() || settings.get("env").is_some_and(|v| !v.is_object()) {
                    return Err(ModeStateError::InvalidConfiguration);
                }
                let shown = super::projection::claude(json!({}), settings, None, None)
                    .map_err(invalid)?
                    .settings;
                let env = shown.get("env").and_then(Value::as_object).cloned().unwrap_or_default();
                let top = shown
                    .as_object()
                    .ok_or(ModeStateError::InvalidConfiguration)?
                    .iter()
                    .filter(|(k, _)| k.as_str() != "env")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                Ok(Self::Claude { env, top })
            }
            AppId::Gemini => {
                if !settings.is_object() || settings.get("env").is_some_and(|v| !v.is_object()) {
                    return Err(ModeStateError::InvalidConfiguration);
                }
                let shown = super::projection::gemini(Map::new(), json!({}), settings, official)
                    .map_err(invalid)?
                    .settings;
                Ok(Self::Gemini {
                    env: shown["env"].as_object().cloned().unwrap_or_default(),
                    config: shown["config"].clone(),
                })
            }
            AppId::GrokBuild => {
                let doc = super::projection::grok_document(
                    toml_edit::DocumentMut::new(),
                    settings,
                    None,
                    official,
                )
                .map_err(invalid)?;
                let document = toml::from_str(&doc.to_string()).map_err(invalid)?;
                Ok(Self::Grok { document })
            }
            _ => Err(ModeStateError::UnsupportedApplication),
        }
    }
}
/// Compare only executor-computed final client projections, after authorizing the
/// state owner. Equality is a planning decision, not proof of native file contents.
/// Task 46 must hold the application guard and validate saved inputs when applying.
pub fn rewrite_decision(
    state: &AppState,
    mode: ProviderMode,
    configuration: &ClientConfiguration,
) -> RewriteDecision {
    let Some(current) = &state.mode else { return RewriteDecision::RewriteRequired };
    let matching_mode = matches!(
        (current.mode, mode),
        (Mode::Direct, ProviderMode::Direct) | (Mode::Proxy, ProviderMode::Proxy)
    );
    if !matching_mode || current.attached != (mode == ProviderMode::Proxy) {
        return RewriteDecision::RewriteRequired;
    }
    let expected = configuration.contract();
    if current.contract.as_ref().is_some_and(|c| {
        c.version == expected.version && c.key == expected.key && c.exclusive == expected.exclusive
    }) {
        RewriteDecision::SkipRewrite
    } else {
        RewriteDecision::RewriteRequired
    }
}
pub fn observe(
    state: &AppState,
    tenant: &TenantScope,
) -> Result<ProviderModeStatus, ModeStateError> {
    if state.pointer.as_ref().is_some_and(|p| p.tenant != *tenant)
        || state.mode.is_some() && state.mode_tenant.as_ref() != Some(tenant)
        || state.stack_tenant.as_ref().is_some_and(|t| t != tenant)
        || !state.stack.is_empty() && state.stack_tenant.as_ref() != Some(tenant)
    {
        return Err(ModeStateError::Ownership);
    }
    let mode = if state.mode.as_ref().is_some_and(|m| m.mode == Mode::Proxy) {
        ProviderMode::Proxy
    } else {
        ProviderMode::Direct
    };
    let direct = state.pointer.as_ref().map(|p| p.id.clone());
    let route = state.mode.as_ref().and_then(|m| m.proxy_route.clone());
    Ok(ProviderModeStatus {
        mode,
        direct_provider: direct.clone(),
        in_use_provider: if mode == ProviderMode::Proxy { route.clone() } else { direct },
        proxy_route: route,
        attached: state.mode.as_ref().is_some_and(|m| m.attached),
        has_contract: state.mode.as_ref().is_some_and(|m| m.contract.is_some()),
        stack: StackStatus {
            enabled: state.stack.enabled,
            active: state.stack.enabled && mode == ProviderMode::Proxy,
            members: state.stack.members.clone(),
        },
        decision: None,
        route_runtime: RouteRuntimeStatus::Unavailable,
    })
}
