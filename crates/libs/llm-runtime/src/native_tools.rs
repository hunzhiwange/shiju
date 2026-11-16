//! Typed protocol conversion. Parsing never changes tool exposure or grants.

// Native call state is owned by the SDK conversion module.

use aisdk::core::{
    LoadableToolSpec, Message, NativeToolSpec, ToolSearchExecution,
    ToolSearchOutput,
};


use model_types::{ToolCall, ToolCallKind};
use tool_catalog::catalog::ToolCatalogSnapshot;
use tool_catalog::discovery::ToolDiscoveryCatalog;

pub use crate::aisdk::native_call_collector::{NativeCallCollector, parse_search_args};

/// Build a dedicated SDK result from a pinned runtime-authorized catalog only.
/// This does not load tools: the next loop stage owns exposure updates.
pub fn native_search_output(
    call: &ToolCall,
    catalog: &ToolCatalogSnapshot,
    enforce_strict: bool,
) -> anyhow::Result<Message> {
    anyhow::ensure!(
        call.kind == ToolCallKind::Search
            && call.namespace.is_none()
            && call.name == "tool_search"
            && !call.id.trim().is_empty(),
        "expected native search call"
    );
    native_search_output_from_specs(call, catalog.specs(), enforce_strict)
}

pub fn native_search_output_from_specs(
    call: &ToolCall,
    specs: Vec<tool_types::ToolSpec>,
    enforce_strict: bool,
) -> anyhow::Result<Message> {
    anyhow::ensure!(
        call.kind == ToolCallKind::Search
            && call.namespace.is_none()
            && call.name == "tool_search"
            && !call.id.trim().is_empty(),
        "expected native search call"
    );
    anyhow::ensure!(
        specs.iter().any(|spec| spec.id == "tool_search" && spec.exposure.is_direct()),
        "tool_search is not authorized for this catalog"
    );
    let args = parse_search_args(serde_json::from_str(&call.arguments)?)?;
    let hits = ToolDiscoveryCatalog::new(specs).search(&args.query, args.limit)?;
    let hits = hits.into_iter().map(|spec| (spec.id.clone(), spec)).collect();
    let tools = super::aisdk::convert::native_request_tools(&hits, enforce_strict)?
        .into_iter()
        .map(|spec| match spec {
            NativeToolSpec::Function(function) => Ok(LoadableToolSpec::Function(function)),
            NativeToolSpec::Namespace(namespace) => Ok(LoadableToolSpec::Namespace(namespace)),
            NativeToolSpec::ToolSearch { .. } => {
                anyhow::bail!("search results cannot contain tool_search")
            }
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(Message::ToolSearchOutput(ToolSearchOutput {
        id: None,
        call_id: Some(call.id.clone()),
        status: "completed".into(),
        execution: ToolSearchExecution::Client,
        tools,
    }))
}

pub use crate::aisdk::native_history::{NativeHistoryItem, normalize_history};
