//! Typed protocol conversion. Parsing never changes tool exposure or grants.

use std::collections::HashMap;

use aisdk::core::{
    LoadableToolSpec, Message, NativeToolSpec, ToolSearchCall, ToolSearchExecution,
    ToolSearchOutput,
};
use serde::Deserialize;
use serde_json::Value;

use super::types::{Error, ToolCall, ToolCallKind};
use crate::tools::catalog::ToolCatalogSnapshot;
use crate::tools::discovery::ToolDiscoveryCatalog;

#[derive(Default)]
pub(crate) struct NativeCallCollector {
    calls: HashMap<String, ToolCall>,
}

impl NativeCallCollector {
    /// Identical Available/Done notifications are one call. Conflicting identities fail closed.
    pub(crate) fn record(&mut self, call: ToolCall) -> Result<Option<ToolCall>, Error> {
        if let Some(previous) = self.calls.get(&call.id) {
            if previous.kind != call.kind
                || previous.namespace != call.namespace
                || previous.name != call.name
                || previous.arguments != call.arguments
            {
                return Err(protocol_error("conflicting tool call identity or arguments"));
            }
            return Ok(None);
        }
        self.calls.insert(call.id.clone(), call.clone());
        Ok(Some(call))
    }

    pub(crate) fn search(
        &mut self,
        call: ToolSearchCall,
        enabled: bool,
    ) -> Result<Option<ToolCall>, Error> {
        if !enabled {
            return Err(protocol_error("native tool search is not authorized for this request"));
        }
        if call.execution != ToolSearchExecution::Client {
            return Err(protocol_error("server tool search cannot execute locally"));
        }
        if call.status.as_deref().is_some_and(|status| status != "completed") {
            return Err(protocol_error("native tool search is not completed"));
        }
        let id = call
            .call_id
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| protocol_error("native tool search requires call_id"))?;
        parse_search_args(call.arguments.clone()).map_err(protocol_error)?;
        self.record(ToolCall {
            kind: ToolCallKind::Search,
            namespace: None,
            id,
            name: "tool_search".into(),
            arguments: call.arguments.to_string(),
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SearchArgs {
    query: String,
    limit: Option<usize>,
}

fn parse_search_args(value: Value) -> anyhow::Result<SearchArgs> {
    let args: SearchArgs = serde_json::from_value(value)?;
    anyhow::ensure!(
        !args.query.trim().is_empty(),
        "query must contain a searchable word or character"
    );
    anyhow::ensure!(
        args.limit
            .is_none_or(|limit| (1..=crate::tools::discovery::MAX_SEARCH_LIMIT).contains(&limit)),
        "invalid search limit"
    );
    Ok(args)
}

fn protocol_error(message: impl std::fmt::Display) -> Error {
    Error::Api(crate::session::message::AssistantError::Unknown { message: message.to_string() })
}

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

pub(crate) fn native_search_output_from_specs(
    call: &ToolCall,
    specs: Vec<crate::tools::ToolSpec>,
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

/// Typed native history. Decoding never changes runtime authorization.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum NativeHistoryItem {
    ToolSearchCall(ToolSearchCall),
    ToolSearchOutput(ToolSearchOutput),
}

impl NativeHistoryItem {
    pub(crate) fn into_message(self) -> Message {
        match self {
            Self::ToolSearchCall(call) => Message::Assistant(aisdk::core::AssistantMessage {
                content:
                    aisdk::core::language_model::LanguageModelResponseContentType::ToolSearchCall(
                        call,
                    ),
                usage: None,
            }),
            Self::ToolSearchOutput(output) => Message::ToolSearchOutput(output),
        }
    }
}

/// Keep complete client search pairs once, without interpreting schemas as grants.
/// A prefix cut may orphan either half; conflicting identities are never replayed.
pub(crate) fn normalize_history(messages: &[Value]) -> anyhow::Result<Vec<Value>> {
    let mut calls = HashMap::<String, (usize, Value)>::new();
    let mut outputs = HashMap::<String, (usize, Value)>::new();
    for (index, message) in messages.iter().enumerate() {
        if message["role"] != "native" {
            continue;
        }
        let content = message["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("native history requires serialized content"))?;
        let item: NativeHistoryItem = serde_json::from_str(content)?;
        let (id, entries) = match item {
            NativeHistoryItem::ToolSearchCall(call) => {
                anyhow::ensure!(
                    call.execution == ToolSearchExecution::Client,
                    "invalid history search execution"
                );
                (call.call_id, &mut calls)
            }
            NativeHistoryItem::ToolSearchOutput(output) => {
                anyhow::ensure!(
                    output.execution == ToolSearchExecution::Client,
                    "invalid history search execution"
                );
                (output.call_id, &mut outputs)
            }
        };
        let id = id
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("native history requires call_id"))?;
        let value: Value = serde_json::from_str(content)?;
        if let Some((_, previous)) = entries.get(&id) {
            anyhow::ensure!(previous == &value, "conflicting native history identity");
        } else {
            entries.insert(id, (index, value));
        }
    }
    let mut keep = std::collections::HashSet::new();
    for (id, (call_index, _)) in calls {
        if let Some((output_index, _)) = outputs.get(&id) {
            anyhow::ensure!(call_index < *output_index, "native history output precedes call");
            keep.insert(call_index);
            keep.insert(*output_index);
        }
    }
    Ok(messages
        .iter()
        .enumerate()
        .filter(|(index, message)| message["role"] != "native" || keep.contains(index))
        .map(|(_, message)| message.clone())
        .collect())
}
