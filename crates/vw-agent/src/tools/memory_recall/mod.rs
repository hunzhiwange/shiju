//! 记忆检索工具
//!
//! 从长期记忆中搜索相关的事实、偏好或上下文。返回按相关性排序的结果，
//! 支持限制返回数量。

use super::traits::{Tool, ToolResult};
use crate::app::agent::memory::Memory;
use async_trait::async_trait;
use serde_json::json;
use std::fmt::Write;
use std::sync::Arc;

/// Let the agent search its own memory
pub struct MemoryRecallTool {
    memory: Arc<dyn Memory>,
}

impl MemoryRecallTool {
    pub fn new(memory: Arc<dyn Memory>) -> Self {
        Self { memory }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for MemoryRecallTool {
    fn name(&self) -> &str {
        "memory_recall"
    }

    fn description(&self) -> &str {
        "Search long-term memory for relevant facts, preferences, or context. Returns scored results ordered by relevance."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Keyword or phrase to search for in memory"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return. Defaults to 5."
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;

        #[allow(clippy::cast_possible_truncation)]
        let limit = args.get("limit").and_then(serde_json::Value::as_u64).map_or(5, |v| v as usize);

        match self.memory.recall(query, limit, None).await {
            Ok(entries) if entries.is_empty() => Ok(ToolResult {
                success: true,
                output: "No memories found matching that query.".into(),
                error: None,
            }),
            Ok(entries) => {
                let mut output = format!("Found {} memories:\n", entries.len());
                for entry in &entries {
                    let score = entry.score.map_or_else(String::new, |s| format!(" [{s:.0}%]"));
                    let _ = writeln!(
                        output,
                        "- [{}] {}: {}{score}",
                        entry.category, entry.key, entry.content
                    );
                }
                Ok(ToolResult { success: true, output, error: None })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Memory recall failed: {e}")),
            }),
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
