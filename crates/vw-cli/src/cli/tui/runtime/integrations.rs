//! Gateway 集成目录的并发加载与稳定 ID 管理边界。

use serde_json::Value;
use vw_gateway_client::vw_api_types::mcp::McpConnectionStatusDto;

use super::gateway::{GatewayUiRuntime, block_on_gateway};
use crate::cli::tui::model::integration::{
    IntegrationAction, IntegrationItem, IntegrationKind, sanitize_integration_text,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct LoadedIntegrationCatalog {
    pub(crate) items: Vec<IntegrationItem>,
    pub(crate) warnings: Vec<String>,
}

pub(crate) async fn load_integrations(runtime: &GatewayUiRuntime) -> LoadedIntegrationCatalog {
    let directory = runtime.directory_value();
    let (skills, plugins, mcp, memory) = tokio::join!(
        runtime.client().skills_get(directory.as_deref()),
        runtime.client().plugins_get(),
        runtime.client().mcp_inspect(),
        runtime.client().memory_query_get(None, None),
    );
    let mut loaded = LoadedIntegrationCatalog::default();

    match skills {
        Ok(entries) => loaded.items.extend(entries.into_iter().map(|entry| IntegrationItem {
            kind: IntegrationKind::Skill,
            id: entry.id.clone(),
            title: entry.title,
            detail: entry.description,
            source: entry.source,
            enabled: entry.enabled,
            can_toggle: entry.installed,
            can_delete: entry.installed && entry.kind != "built-in",
            prompt_prefix: Some(format!("Use the `{}` skill.", entry.id)),
            actions: toggle_actions(entry.enabled),
        })),
        Err(error) => loaded.warnings.push(sanitized_warning("skills", error)),
    }
    match plugins {
        Ok(entries) => loaded.items.extend(entries.into_iter().map(|entry| IntegrationItem {
            kind: IntegrationKind::Plugin,
            id: entry.id,
            title: entry.display_name,
            detail: format!("{} · {} components", entry.version, entry.component_count),
            source: "gateway:plugin".to_string(),
            enabled: entry.enabled,
            can_toggle: true,
            can_delete: true,
            prompt_prefix: None,
            actions: toggle_actions(entry.enabled),
        })),
        Err(error) => loaded.warnings.push(sanitized_warning("plugins", error)),
    }
    match mcp {
        Ok(response) => loaded.items.extend(response.servers.into_iter().map(|server| {
            let enabled = server.enabled
                && matches!(server.connection_status, McpConnectionStatusDto::Connected);
            IntegrationItem {
                kind: IntegrationKind::Mcp,
                id: server.name.clone(),
                title: server.name,
                detail: format!(
                    "{} · {} tools · {} resources",
                    server.transport,
                    server.tool_count.unwrap_or_default(),
                    server.resource_count.unwrap_or_default(),
                ),
                source: "gateway:mcp".to_string(),
                enabled,
                can_toggle: false,
                can_delete: false,
                prompt_prefix: None,
                actions: Vec::new(),
            }
        })),
        Err(error) => loaded.warnings.push(sanitized_warning("mcp", error)),
    }
    match memory {
        Ok(value) => loaded.items.extend(memory_items(&value)),
        Err(error) => loaded.warnings.push(sanitized_warning("memory", error)),
    }

    loaded.items.sort_by(|left, right| {
        (left.kind, &left.id, source_priority(&left.source)).cmp(&(
            right.kind,
            &right.id,
            source_priority(&right.source),
        ))
    });
    loaded.items.dedup_by(|right, left| right.kind == left.kind && right.id == left.id);
    loaded
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_integrations_blocking(
    runtime: &GatewayUiRuntime,
) -> LoadedIntegrationCatalog {
    block_on_gateway(async { Ok(load_integrations(runtime).await) }).unwrap_or_else(|error| {
        LoadedIntegrationCatalog {
            items: Vec::new(),
            warnings: vec![sanitized_warning("catalog", error)],
        }
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn set_integration_enabled_blocking(
    runtime: &GatewayUiRuntime,
    item: &IntegrationItem,
    enabled: bool,
) -> Result<(), String> {
    if !item.can_toggle || item.id.trim().is_empty() {
        return Err("integration does not expose a stable toggle operation".to_string());
    }
    block_on_gateway(async {
        match item.kind {
            IntegrationKind::Skill => runtime
                .client()
                .skill_set_enabled(runtime.directory_value().as_deref(), item.id.as_str(), enabled)
                .await
                .map(|_| ()),
            IntegrationKind::Plugin => runtime
                .client()
                .plugin_set_enabled(item.id.as_str(), enabled)
                .await
                .map(|_| ()),
            _ => Err("integration kind cannot be toggled".to_string()),
        }
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn delete_integration_blocking(
    runtime: &GatewayUiRuntime,
    item: &IntegrationItem,
) -> Result<(), String> {
    if !item.can_delete || item.id.trim().is_empty() {
        return Err("integration does not expose a stable delete operation".to_string());
    }
    block_on_gateway(async {
        match item.kind {
            IntegrationKind::Skill => runtime
                .client()
                .skill_delete(runtime.directory_value().as_deref(), item.id.as_str())
                .await
                .map(|_| ()),
            IntegrationKind::Plugin => runtime
                .client()
                .plugin_delete(item.id.as_str())
                .await
                .map(|_| ()),
            _ => Err("integration kind cannot be deleted".to_string()),
        }
    })
}

fn toggle_actions(enabled: bool) -> Vec<IntegrationAction> {
    vec![IntegrationAction {
        label: if enabled { "disable" } else { "enable" }.to_string(),
        enabled: true,
        disabled_reason: None,
    }]
}

fn memory_items(value: &Value) -> Vec<IntegrationItem> {
    value
        .get("entries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let id = entry
                .get("key")
                .or_else(|| entry.get("id"))
                .and_then(Value::as_str)?
                .trim();
            if id.is_empty() {
                return None;
            }
            let detail = entry
                .get("content")
                .or_else(|| entry.get("preview"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            Some(IntegrationItem {
                kind: IntegrationKind::Memory,
                id: id.to_string(),
                title: entry.get("title").and_then(Value::as_str).unwrap_or(id).to_string(),
                detail: sanitize_integration_text(detail),
                source: entry
                    .get("scope")
                    .and_then(Value::as_str)
                    .unwrap_or("gateway:memory")
                    .to_string(),
                enabled: true,
                can_toggle: false,
                can_delete: false,
                prompt_prefix: None,
                actions: Vec::new(),
            })
        })
        .collect()
}

fn source_priority(source: &str) -> u8 {
    let source = source.to_ascii_lowercase();
    if source.contains("project") || source.contains("local") {
        0
    } else if source.contains("plugin") {
        1
    } else if source.contains("built") {
        2
    } else {
        3
    }
}

fn sanitized_warning(source: &str, error: String) -> String {
    format!("{source}: {}", sanitize_integration_text(&error))
}
