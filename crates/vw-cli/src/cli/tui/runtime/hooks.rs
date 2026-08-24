//! Gateway Hook 声明、信任 mutation 与安全审计边界。

use vw_gateway_client::{HookAuditEntryDto, HookCatalogEntryDto, HookMutationResponseDto};
use vw_tui::V3SelectionItem;

use super::gateway::{GatewayUiRuntime, block_on_gateway};
use crate::cli::tui::model::integration::sanitize_integration_text;

const MAX_HOOK_ENTRIES: usize = 100;
const MAX_PLUGIN_ID_CHARS: usize = 128;
const MAX_HOOK_HASH_CHARS: usize = 256;
const MAX_EVENT_CHARS: usize = 80;
const MAX_MODE_CHARS: usize = 32;
const MAX_TIMESTAMP_CHARS: usize = 64;
const MAX_TOOL_NAME_CHARS: usize = 96;
const MAX_ERROR_CHARS: usize = 320;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookCatalogItem {
    pub(crate) plugin_id: String,
    pub(crate) event: String,
    pub(crate) mode: String,
    pub(crate) hook_hash: String,
    pub(crate) trusted: bool,
}

/// 仅保存后续信任操作所需稳定凭据及可安全展示的声明字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookCatalogView {
    items: Vec<HookCatalogItem>,
    malformed_count: usize,
    truncated: bool,
}

impl HookCatalogView {
    pub(crate) fn find(
        &self,
        plugin_id: &str,
        hook_hash: &str,
    ) -> Option<&HookCatalogItem> {
        self.items
            .iter()
            .find(|item| item.plugin_id == plugin_id && item.hook_hash == hook_hash)
    }

    pub(crate) fn into_selection_items(self) -> Vec<V3SelectionItem> {
        let mut items = self
            .items
            .into_iter()
            .map(|item| {
                let trust = if item.trusted { "trusted" } else { "pending" };
                let stable_id = serde_json::json!({
                    "plugin_id": item.plugin_id,
                    "hook_hash": item.hook_hash,
                })
                .to_string();
                V3SelectionItem {
                    id: stable_id,
                    label: format!("{} · {}", item.plugin_id, item.event),
                    description: Some(format!(
                        "{trust} · mode {} · hash {}",
                        item.mode, item.hook_hash
                    )),
                    search_value: Some(format!(
                        "{} {} {} {}",
                        item.plugin_id, item.event, item.mode, item.hook_hash
                    )),
                    disabled: true,
                    disabled_reason: Some(
                        "使用 /hooks trust|revoke <plugin-id> <hash> 管理信任".to_string(),
                    ),
                    ..V3SelectionItem::default()
                }
            })
            .collect::<Vec<_>>();
        if self.malformed_count > 0 {
            items.push(status_item(format!(
                "Ignored {} malformed Hook declaration(s)",
                self.malformed_count
            )));
        }
        if self.truncated {
            items.push(status_item(format!(
                "Hook catalog truncated to {MAX_HOOK_ENTRIES} entries"
            )));
        }
        if items.is_empty() {
            items.push(status_item("未发现 Hook 声明".to_string()));
        }
        items
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookMutationRefresh {
    pub(crate) catalog: HookCatalogView,
    pub(crate) audit: HookAuditView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HookAuditItem {
    timestamp: String,
    tool_name: String,
    duration_ms: u64,
    success: bool,
}

/// 只保留 Gateway bounded audit DTO 的安全字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HookAuditView {
    items: Vec<HookAuditItem>,
    malformed_count: usize,
    truncated: bool,
}

impl HookAuditView {
    pub(crate) fn render_text(&self) -> String {
        let mut lines = vec!["Hook audit".to_string()];
        if self.items.is_empty() {
            lines.push("未发现 Hook 审计记录".to_string());
        }
        lines.extend(self.items.iter().map(|item| {
            format!(
                "{} · {} · {} ms · {}",
                item.timestamp,
                item.tool_name,
                item.duration_ms,
                if item.success { "success" } else { "failed" }
            )
        }));
        if self.malformed_count > 0 {
            lines.push(format!(
                "Ignored {} malformed Hook audit record(s)",
                self.malformed_count
            ));
        }
        if self.truncated {
            lines.push(format!(
                "Hook audit truncated to {MAX_HOOK_ENTRIES} entries"
            ));
        }
        lines.join("\n")
    }
}

pub(crate) fn load_hook_catalog_blocking(
    runtime: &GatewayUiRuntime,
) -> Result<HookCatalogView, String> {
    block_on_gateway(async {
        runtime
            .client()
            .hooks_get()
            .await
            .map(hook_catalog_view)
            .map_err(|error| sanitize_hook_text(error.as_str(), MAX_ERROR_CHARS))
    })
}

pub(crate) fn load_hook_audit_blocking(
    runtime: &GatewayUiRuntime,
) -> Result<HookAuditView, String> {
    block_on_gateway(async {
        runtime
            .client()
            .hook_audit_get()
            .await
            .map(hook_audit_view)
            .map_err(|error| sanitize_hook_text(error.as_str(), MAX_ERROR_CHARS))
    })
}

pub(crate) fn mutate_hook_and_refresh_blocking(
    runtime: &GatewayUiRuntime,
    plugin_id: &str,
    hook_hash: &str,
    trusted: bool,
) -> Result<HookMutationRefresh, String> {
    let response = block_on_gateway(async {
        if trusted {
            runtime.client().hook_trust(plugin_id, hook_hash).await
        } else {
            runtime.client().hook_revoke(plugin_id, hook_hash).await
        }
    })
    .map_err(|error| sanitize_hook_text(error.as_str(), MAX_ERROR_CHARS))?;
    validate_mutation_response(&response, plugin_id, hook_hash, trusted)?;

    let catalog = load_hook_catalog_blocking(runtime)?;
    let audit = load_hook_audit_blocking(runtime)?;
    Ok(HookMutationRefresh { catalog, audit })
}

fn validate_mutation_response(
    response: &HookMutationResponseDto,
    plugin_id: &str,
    hook_hash: &str,
    trusted: bool,
) -> Result<(), String> {
    if response.plugin_id != plugin_id
        || response.hook_hash != hook_hash
        || response.trusted != trusted
    {
        return Err("Gateway returned a mismatched Hook mutation response".to_string());
    }
    Ok(())
}

fn hook_catalog_view(entries: Vec<HookCatalogEntryDto>) -> HookCatalogView {
    let truncated = entries.len() > MAX_HOOK_ENTRIES;
    let mut malformed_count = 0;
    let items = entries
        .into_iter()
        .take(MAX_HOOK_ENTRIES)
        .filter_map(|entry| match hook_catalog_item(entry) {
            Some(item) => Some(item),
            None => {
                malformed_count += 1;
                None
            }
        })
        .collect();
    HookCatalogView { items, malformed_count, truncated }
}

fn hook_catalog_item(entry: HookCatalogEntryDto) -> Option<HookCatalogItem> {
    if !stable_credential(entry.plugin_id.as_str(), MAX_PLUGIN_ID_CHARS)
        || !stable_credential(entry.hook_hash.as_str(), MAX_HOOK_HASH_CHARS)
    {
        return None;
    }
    let event = sanitize_hook_text(entry.event.as_str(), MAX_EVENT_CHARS);
    let mode = sanitize_hook_text(entry.mode.as_str(), MAX_MODE_CHARS);
    if event.is_empty() || mode.is_empty() {
        return None;
    }
    Some(HookCatalogItem {
        plugin_id: entry.plugin_id,
        event,
        mode,
        hook_hash: entry.hook_hash,
        trusted: entry.trusted,
    })
}

fn hook_audit_view(entries: Vec<HookAuditEntryDto>) -> HookAuditView {
    let truncated = entries.len() > MAX_HOOK_ENTRIES;
    let mut malformed_count = 0;
    let items = entries
        .into_iter()
        .take(MAX_HOOK_ENTRIES)
        .filter_map(|entry| {
            let timestamp = sanitize_hook_text(entry.timestamp.as_str(), MAX_TIMESTAMP_CHARS);
            let tool_name = sanitize_hook_text(entry.tool_name.as_str(), MAX_TOOL_NAME_CHARS);
            if timestamp.is_empty() || tool_name.is_empty() {
                malformed_count += 1;
                return None;
            }
            Some(HookAuditItem {
                timestamp,
                tool_name,
                duration_ms: entry.duration_ms,
                success: entry.success,
            })
        })
        .collect();
    HookAuditView { items, malformed_count, truncated }
}

fn stable_credential(value: &str, max_chars: usize) -> bool {
    let count = value.chars().count();
    count > 0
        && count <= max_chars
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '-' | '_' | '.' | '/' | ':' | '@')
        })
}

fn sanitize_hook_text(value: &str, max_chars: usize) -> String {
    let sanitized = sanitize_integration_text(value);
    if sanitized.chars().count() <= max_chars {
        return sanitized;
    }
    if max_chars <= 3 {
        return sanitized.chars().take(max_chars).collect();
    }
    format!(
        "{}...",
        sanitized.chars().take(max_chars.saturating_sub(3)).collect::<String>()
    )
}

fn status_item(label: String) -> V3SelectionItem {
    V3SelectionItem {
        id: format!("status:{label}"),
        label,
        disabled: true,
        disabled_reason: Some("只读状态".to_string()),
        ..V3SelectionItem::default()
    }
}
