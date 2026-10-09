//! 从 Gateway 公开契约加载活动 TUI 的会话能力。

use std::collections::HashSet;

use serde_json::Value;

use super::gateway::{GatewayUiRuntime, block_on_gateway};
use crate::cli::tui::model::{
    AcpBackendOption, DelegateAgentOption, PermissionMode, ReasoningEffort,
    SessionCapabilityMatrix,
};

/// Gateway 能力加载结果；失败来源保留为可展示警告，矩阵使用安全最小值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadedSessionCapabilities {
    pub(crate) matrix: SessionCapabilityMatrix,
    pub(crate) default_model: Option<String>,
    pub(crate) warnings: Vec<String>,
}

/// 分别读取 provider、目录配置与 ACP 配置，单个来源失败不会扩大可用能力。
pub(crate) async fn load_session_capabilities(
    runtime: &GatewayUiRuntime,
) -> LoadedSessionCapabilities {
    let mut matrix = SessionCapabilityMatrix {
        permission_modes: vec![PermissionMode::Default, PermissionMode::Plan],
        ..SessionCapabilityMatrix::default()
    };
    let mut warnings = Vec::new();
    let mut default_model = None;
    let directory = runtime.directory_value();

    match runtime.client().provider_list(directory.as_deref()).await {
        Ok(response) => {
            let mut provider_defaults = response.default.into_iter().collect::<Vec<_>>();
            provider_defaults.sort_by(|(left, _), (right, _)| left.cmp(right));
            let connected = response.connected.into_iter().collect::<HashSet<_>>();
            let mut models = Vec::new();
            let mut supports_reasoning = false;
            for provider in response.all {
                if !connected.contains(provider.id.as_str()) {
                    continue;
                }
                let mut provider_models = provider.models.into_values().collect::<Vec<_>>();
                provider_models.sort_by(|left, right| left.id.cmp(&right.id));
                for model in provider_models {
                    supports_reasoning |= model.capabilities.reasoning;
                    models.push(format!("{}/{}", provider.id, model.id));
                }
            }
            models.sort();
            models.dedup();
            matrix.models = models;
            default_model = provider_defaults.into_iter().find_map(|(provider, model)| {
                let qualified =
                    if model.contains('/') { model } else { format!("{provider}/{model}") };
                matrix.models.iter().any(|candidate| candidate == &qualified).then_some(qualified)
            });
            matrix.supports_thinking = supports_reasoning;
            if supports_reasoning {
                matrix.efforts = vec![
                    ReasoningEffort::Minimal,
                    ReasoningEffort::Low,
                    ReasoningEffort::Medium,
                    ReasoningEffort::High,
                    ReasoningEffort::XHigh,
                ];
            }
            if matrix.models.is_empty() {
                warnings.push("Gateway 未返回已连接 provider 的可选模型".to_string());
            }
        }
        Err(error) => warnings.push(format!("加载 Gateway provider 能力失败：{error}")),
    }

    match runtime.client().config_get(directory.as_deref()).await {
        Ok(config) => {
            default_model = configured_default_model(&config)
                .filter(|model| matrix.models.iter().any(|candidate| candidate == model))
                .or(default_model);
            matrix.delegate_agents = configured_delegate_agents(&config);
            if config_allows_dangerous_access(&config) {
                matrix.permission_modes.push(PermissionMode::Bypass);
            }
        }
        Err(error) => warnings.push(format!("加载 Gateway 会话配置失败：{error}")),
    }

    match runtime.enabled_acp_backend_keys().await {
        Ok(keys) => {
            matrix.acp_backends.extend(keys.into_iter().map(|key| AcpBackendOption {
                label: key.clone(),
                key: Some(key),
            }));
        }
        Err(error) => warnings.push(format!("加载 Gateway ACP 能力失败：{error}")),
    }

    LoadedSessionCapabilities { matrix, default_model, warnings }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_session_capabilities_blocking(
    runtime: &GatewayUiRuntime,
) -> LoadedSessionCapabilities {
    match block_on_gateway(async { Ok(load_session_capabilities(runtime).await) }) {
        Ok(loaded) => loaded,
        Err(error) => LoadedSessionCapabilities {
            matrix: SessionCapabilityMatrix::default(),
            default_model: None,
            warnings: vec![format!("加载 Gateway 会话能力失败：{error}")],
        },
    }
}

fn configured_default_model(config: &Value) -> Option<String> {
    config
        .get("default_model")
        .or_else(|| config.get("model"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn configured_delegate_agents(config: &Value) -> Vec<DelegateAgentOption> {
    let mut agents = vec![DelegateAgentOption::main()];
    let Some(configured) = config.get("agents").and_then(Value::as_object) else {
        return agents;
    };
    let mut entries = configured.iter().collect::<Vec<_>>();
    entries.sort_by_key(|(name, _)| *name);
    for (key, value) in entries {
        if value.get("enabled").and_then(Value::as_bool) == Some(false)
            || value.get("hidden").and_then(Value::as_bool) == Some(true)
        {
            continue;
        }
        let label = value
            .get("label")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(key);
        agents.push(DelegateAgentOption { key: Some(key.clone()), label: label.to_string() });
    }
    agents
}

fn config_allows_dangerous_access(config: &Value) -> bool {
    let autonomy = config.get("autonomy");
    autonomy
        .and_then(|value| value.get("level"))
        .and_then(Value::as_str)
        .is_some_and(|level| level.eq_ignore_ascii_case("full"))
        && autonomy
            .and_then(|value| value.get("workspace_only"))
            .and_then(Value::as_bool)
            == Some(false)
}
