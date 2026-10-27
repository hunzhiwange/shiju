//! 网关插件钩子的发现、信任变更与命令审计接口。
//!
//! 信任变更依据当前清单哈希，避免在插件内容变化后沿用旧授权。
//!
//! Gateway plugin hook management and command audit handlers.

use anyhow::{Context, Result, bail};
use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use std::path::{Path, PathBuf};
use vw_gateway_client::{
    HookAuditEntryDto, HookCatalogEntryDto, HookMutationRequestDto, HookMutationResponseDto,
};

use crate::app::agent::gateway::{ApiError, AppState};
use crate::plugins::{
    PluginDiscoveryOptions, PluginHook, PluginHookEvent, PluginHookTrustStatus,
    PluginHookTrustStore, discover_plugins, load_plugin_hooks_manifest, plugin_hook_trust_file,
    plugin_hook_trust_hash,
};

/// 构建挂载在网关 `/v1` 下的钩子目录、信任和审计路由。
pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/hooks", get(catalog_get))
        .route("/hooks/trust", post(trust_post))
        .route("/hooks/revoke", post(revoke_post))
        .route("/hooks/audit", get(audit_get))
}

async fn catalog_get(
    State(state): State<AppState>,
) -> Result<Json<Vec<HookCatalogEntryDto>>, ApiError> {
    let config = state.config.lock().clone();
    let items = tokio::task::spawn_blocking(move || collect_hook_catalog(&config))
        .await
        .map_err(|error| ApiError::internal(format!("hook catalog task failed: {error}")))?
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok(Json(items))
}

async fn trust_post(
    State(state): State<AppState>,
    Json(request): Json<HookMutationRequestDto>,
) -> Result<Json<HookMutationResponseDto>, ApiError> {
    mutate_trust(state, request, true).await.map(Json)
}

async fn revoke_post(
    State(state): State<AppState>,
    Json(request): Json<HookMutationRequestDto>,
) -> Result<Json<HookMutationResponseDto>, ApiError> {
    mutate_trust(state, request, false).await.map(Json)
}

async fn audit_get(
    State(state): State<AppState>,
) -> Result<Json<Vec<HookAuditEntryDto>>, ApiError> {
    let runtime = state
        .hook_runtime
        .as_ref()
        .ok_or_else(|| ApiError::internal("hook runtime is unavailable"))?;
    let entries = runtime
        .audit_store()
        .entries()
        .into_iter()
        .map(|entry| HookAuditEntryDto {
            timestamp: entry.timestamp.to_rfc3339(),
            tool_name: entry.tool_name,
            duration_ms: entry.duration_ms,
            success: entry.success,
        })
        .collect();
    Ok(Json(entries))
}

async fn mutate_trust(
    state: AppState,
    request: HookMutationRequestDto,
    trusted: bool,
) -> Result<HookMutationResponseDto, ApiError> {
    let plugin_id = required_field("plugin_id", &request.plugin_id)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let hook_hash = required_field("hook_hash", &request.hook_hash)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let config = state.config.lock().clone();
    let runtime = state
        .hook_runtime
        .clone()
        .ok_or_else(|| ApiError::internal("hook runtime is unavailable"))?;
    let plugin_id_for_task = plugin_id.clone();
    let hook_hash_for_task = hook_hash.clone();

    let candidate = tokio::task::spawn_blocking(move || {
        let (hook, trust_path) =
            resolve_current_hook(&config, &plugin_id_for_task, &hook_hash_for_task)?;
        let mut store = PluginHookTrustStore::load_or_default(&trust_path)?;
        let was_trusted =
            store.review_status(&plugin_id_for_task, &hook) == PluginHookTrustStatus::Trusted;
        if trusted {
            store.trust_hook(&plugin_id_for_task, &hook)?;
        } else {
            store.revoke_hash(&hook_hash_for_task)?;
        }
        match runtime.build_candidate(&config) {
            Ok(candidate) => Ok(candidate),
            Err(error) => {
                if was_trusted {
                    store.trust_hook(&plugin_id_for_task, &hook)?;
                } else {
                    store.revoke_hash(&hook_hash_for_task)?;
                }
                Err(error)
            }
        }
    })
    .await
    .map_err(|error| ApiError::internal(format!("hook trust task failed: {error}")))?
    .map_err(|error| ApiError::bad_request(error.to_string()))?;

    state.hook_runtime.as_ref().expect("runtime checked before mutation").replace(candidate);
    Ok(HookMutationResponseDto { plugin_id, hook_hash, trusted })
}

fn collect_hook_catalog(config: &crate::config::Config) -> Result<Vec<HookCatalogEntryDto>> {
    let options = discovery_options(config);
    let discovery = discover_plugins(&options).context("failed to discover plugin hooks")?;
    let store =
        PluginHookTrustStore::load_or_default(&plugin_hook_trust_file(&options.config_dir))?;
    let mut items = Vec::new();

    for plugin in discovery.installed {
        for hook in load_hooks(&plugin, &options.workspace_dir)? {
            items.push(HookCatalogEntryDto {
                plugin_id: plugin.manifest.name.clone(),
                event: hook.event.as_str().to_string(),
                command: hook.command.clone(),
                args: hook.args.clone(),
                working_dir: hook.working_dir.to_string_lossy().into_owned(),
                mode: hook.mode.as_str().to_string(),
                hook_hash: plugin_hook_trust_hash(&plugin.manifest.name, &hook),
                trusted: store.review_status(&plugin.manifest.name, &hook)
                    == PluginHookTrustStatus::Trusted,
            });
        }
    }
    items.sort_by(|left, right| {
        left.plugin_id
            .cmp(&right.plugin_id)
            .then(left.event.cmp(&right.event))
            .then(left.hook_hash.cmp(&right.hook_hash))
    });
    Ok(items)
}

fn resolve_current_hook(
    config: &crate::config::Config,
    plugin_id: &str,
    hook_hash: &str,
) -> Result<(PluginHook, PathBuf)> {
    if !config.hooks.enabled || !config.hooks.plugins.enabled {
        bail!("plugin hooks are disabled");
    }
    let options = discovery_options(config);
    let discovery = discover_plugins(&options).context("failed to discover plugin hooks")?;
    let plugin = discovery
        .enabled
        .into_iter()
        .find(|plugin| plugin.manifest.name == plugin_id)
        .ok_or_else(|| anyhow::anyhow!("current enabled plugin hook not found"))?;
    let hook = load_hooks(&plugin, &options.workspace_dir)?
        .into_iter()
        .find(|hook| plugin_hook_trust_hash(plugin_id, hook) == hook_hash)
        .ok_or_else(|| anyhow::anyhow!("current enabled plugin hook hash does not match"))?;
    if !matches!(
        hook.event,
        PluginHookEvent::BeforeToolCall
            | PluginHookEvent::AfterToolCall
            | PluginHookEvent::BeforeLlmCall
    ) {
        bail!("plugin hook event is not executable by the current runtime");
    }
    Ok((hook, plugin_hook_trust_file(&options.config_dir)))
}

fn load_hooks(
    plugin: &crate::plugins::DiscoveredPlugin,
    workspace_dir: &Path,
) -> Result<Vec<PluginHook>> {
    let mut hooks = Vec::new();
    for path in &plugin.manifest.hooks {
        hooks.extend(
            load_plugin_hooks_manifest(&path.canonical_path, &plugin.manifest.root, workspace_dir)
                .with_context(|| {
                    format!("failed to load hooks for plugin '{}'", plugin.manifest.name)
                })?,
        );
    }
    Ok(hooks)
}

fn discovery_options(config: &crate::config::Config) -> PluginDiscoveryOptions {
    PluginDiscoveryOptions::new(
        config.workspace_dir.clone(),
        config_dir(config),
        config.plugins.clone(),
    )
}

fn config_dir(config: &crate::config::Config) -> PathBuf {
    config
        .config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| config.workspace_dir.clone())
}

fn required_field(name: &str, value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        bail!("{name} is required");
    }
    Ok(value.to_string())
}

#[cfg(test)]
#[path = "hooks_tests.rs"]
mod hooks_tests;
