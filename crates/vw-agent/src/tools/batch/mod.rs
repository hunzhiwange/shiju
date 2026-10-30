//! 批量工具调用工具
//!
//! 在单个调用中执行多个工具调用，支持并行执行。
//! 适合需要同时执行多个独立操作的场景。

use super::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

const MAX_CALLS: usize = 25;
const MAX_PARALLEL_CALLS: usize = 4;

#[derive(Debug, Clone, Deserialize)]
struct ToolCall {
    tool: String,
    #[serde(default, alias = "parameters")]
    args: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct Args {
    #[serde(alias = "calls")]
    #[serde(alias = "toolCalls")]
    tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone)]
struct CallResult {
    tool: String,
    success: bool,
    output: Option<String>,
    error: Option<String>,
}

pub struct BatchTool {
    tools: Arc<Vec<Arc<dyn Tool>>>,
}

impl BatchTool {
    pub fn new(tools: Arc<Vec<Arc<dyn Tool>>>) -> Self {
        Self { tools }
    }

    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "tool_calls": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": true,
                        "properties": {
                            "tool": { "type": "string", "description": "Name of the tool to execute" }
                        },
                        "required": ["tool"]
                    }
                }
            },
            "additionalProperties": false,
            "required": ["tool_calls"]
        })
    }

    fn available_tool_names(&self) -> Vec<String> {
        let context = super::context::current_tool_use_context();
        self.tools
            .iter()
            .map(|tool| tool.spec().id)
            .filter(|id| context.as_ref().is_none_or(|context| context.tool_is_exposed(id)))
            .collect()
    }

    fn find_tool(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.iter().find(|tool| tool.spec().id == name).cloned()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for BatchTool {
    fn name(&self) -> &str {
        "batch"
    }

    fn description(&self) -> &str {
        include_str!("batch.txt")
    }

    fn parameters_schema(&self) -> serde_json::Value {
        Self::schema()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let args: Args = match serde_json::from_value(args) {
            Ok(value) => value,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("batch 输入参数无效: {e}")),
                });
            }
        };

        if args.tool_calls.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(
                    "batch 工具参数无效：\n  - tool_calls：至少提供一个工具调用\n\n期望的 payload 格式：\n  [{\"tool\": \"tool_name\", \"args\": {...}}, {...}]"
                        .to_string(),
                ),
            });
        }

        let disallowed: HashSet<&'static str> = HashSet::from(["batch"]);
        let mut all_calls = args.tool_calls;
        let discarded_calls =
            if all_calls.len() > MAX_CALLS { all_calls.split_off(MAX_CALLS) } else { Vec::new() };

        let available_tools = self.available_tool_names();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_PARALLEL_CALLS));
        let futures = all_calls.into_iter().map(|call| {
            let semaphore = Arc::clone(&semaphore);
            let disallowed = disallowed.clone();
            let available_tools = available_tools.clone();
            async move {
                let _permit = semaphore.acquire_owned().await.ok();
                let ToolCall { tool, args } = call;

                if disallowed.contains(tool.as_str()) {
                    return CallResult {
                        tool: tool.clone(),
                        success: false,
                        output: None,
                        error: Some(format!(
                            "工具 '{}' 不允许在 batch 中调用。禁用工具：{}",
                            tool,
                            disallowed.iter().copied().collect::<Vec<_>>().join(", ")
                        )),
                    };
                }

                let Some(target) = self.find_tool(&tool) else {
                    return CallResult {
                        tool: tool.clone(),
                        success: false,
                        output: None,
                        error: Some(format!(
                            "工具 '{}' 不在注册表中。请直接单独调用可用工具。可用工具：{}",
                            tool,
                            available_tools.join(", ")
                        )),
                    };
                };

                let params = args.unwrap_or_else(|| serde_json::Value::Object(Default::default()));
                let execution = if let Some(context) = super::context::current_tool_use_context() {
                    // Keep inner calls inside the same authorization, exposure,
                    // permission and hook chain as ordinary runtime calls.
                    let registry =
                        super::toolset::boxed_registry_from_arcs(self.tools.as_ref().clone());
                    super::execute_tool_from_registry(&registry, &tool, params, context)
                        .await
                        .map(|executed| ToolResult {
                            success: executed.result.is_success(),
                            output: executed.result.model_text(),
                            error: executed.result.error_text(),
                        })
                        .map_err(anyhow::Error::new)
                } else {
                    target.execute(params).await
                };
                match execution {
                    Ok(result) => {
                        if result.success {
                            CallResult {
                                tool,
                                success: true,
                                output: Some(result.output),
                                error: None,
                            }
                        } else {
                            CallResult {
                                tool,
                                success: false,
                                output: None,
                                error: Some(result.error.unwrap_or(result.output)),
                            }
                        }
                    }
                    Err(e) => CallResult {
                        tool,
                        success: false,
                        output: None,
                        error: Some(format!("工具执行失败: {e}")),
                    },
                }
            }
        });

        let mut results = futures_util::future::join_all(futures).await;

        for call in discarded_calls {
            results.push(CallResult {
                tool: call.tool,
                success: false,
                output: None,
                error: Some("batch 最多允许 25 个工具调用".to_string()),
            });
        }

        let successful = results.iter().filter(|result| result.success).count();
        let failed = results.len().saturating_sub(successful);
        let output_message = if failed > 0 {
            format!("已成功执行 {}/{} 个工具，失败 {} 个。", successful, results.len(), failed)
        } else {
            format!(
                "全部 {} 个工具均执行成功。\n\n后续也可以继续使用 batch 工具以获得更高的执行效率。",
                successful
            )
        };

        let mut out = vec![output_message];
        let mut success_outputs = results
            .iter()
            .filter(|result| result.success)
            .filter_map(|result| result.output.as_ref())
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if !success_outputs.is_empty() {
            out.push(String::new());
            out.append(&mut success_outputs);
        }

        let errors = results
            .iter()
            .filter(|result| !result.success)
            .filter_map(|result| result.error.as_ref().map(|error| (result.tool.as_str(), error)))
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            out.push(String::new());
            out.push("错误：".to_string());
            for (tool, error) in errors {
                out.push(format!("- {}: {}", tool, error));
            }
        }

        Ok(ToolResult { success: true, output: out.join("\n\n"), error: None })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
