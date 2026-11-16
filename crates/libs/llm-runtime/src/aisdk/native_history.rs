//! Typed native history normalization without granting tool exposure.
use std::collections::HashMap;
use serde_json::Value;
use aisdk::core::{Message, ToolSearchCall, ToolSearchOutput, ToolSearchExecution};

/// Typed native history. Decoding never changes runtime authorization.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NativeHistoryItem {
    ToolSearchCall(ToolSearchCall),
    ToolSearchOutput(ToolSearchOutput),
}

impl NativeHistoryItem {
    pub fn into_message(self) -> Message {
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
pub fn normalize_history(messages: &[Value]) -> anyhow::Result<Vec<Value>> {
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
