//! Native call identity and search argument validation.
use std::collections::HashMap;
use serde::Deserialize;
use serde_json::Value;
use aisdk::core::{ToolSearchCall, ToolSearchExecution};
use llm_runtime::Error;
use model_types::ToolCallKind;
use crate::processor_events::ToolCall;
#[derive(Default)]
pub struct NativeCallCollector {
    calls: HashMap<String, ToolCall>,
}

impl NativeCallCollector {
    /// Identical Available/Done notifications are one call. Conflicting identities fail closed.
    pub fn record(&mut self, call: ToolCall) -> Result<Option<ToolCall>, Error> {
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

    pub fn search(
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
pub struct SearchArgs {
    pub query: String,
    pub limit: Option<usize>,
}

pub fn parse_search_args(value: Value) -> anyhow::Result<SearchArgs> {
    let args: SearchArgs = serde_json::from_value(value)?;
    anyhow::ensure!(
        !args.query.trim().is_empty(),
        "query must contain a searchable word or character"
    );
    anyhow::ensure!(
        args.limit
            .is_none_or(|limit| (1..=tool_catalog::discovery::MAX_SEARCH_LIMIT).contains(&limit)),
        "invalid search limit"
    );
    Ok(args)
}

fn protocol_error(message: impl std::fmt::Display) -> Error {
    Error::Api(message_types::AssistantError::Unknown { message: message.to_string() })
}
