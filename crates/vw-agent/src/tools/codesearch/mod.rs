//! 代码搜索工具
//!
//! 使用 Exa AI 搜索引擎在代码库中搜索相关代码和信息。
//! 支持自然语言查询和返回内容大小控制。

use super::traits::{Tool, ToolResult};
use super::url_validation::{DomainPolicy, UrlSchemePolicy, validate_url};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

const EXA_MCP_URL: &str = "https://mcp.exa.ai/mcp";
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_TOKENS: u32 = 5_000;
const MIN_TOKENS: u32 = 1_000;
const MAX_TOKENS: u32 = 50_000;

#[derive(Debug, Clone, Deserialize)]
struct Args {
    query: Option<String>,
    information_request: Option<String>,
    #[serde(rename = "tokensNum")]
    tokens_num: Option<u32>,
}

pub struct CodeSearchTool {
    security: Arc<SecurityPolicy>,
}

impl CodeSearchTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }

    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Query for API, library, or SDK context. Examples: 'React useState hook examples' and 'Python pandas dataframe filtering'."
                },
                "information_request": {
                    "type": "string",
                    "description": "Compatibility field equivalent to query."
                },
                "tokensNum": {
                    "type": "number",
                    "description": "Number of tokens to return (1000-50000). Defaults to 5000."
                }
            },
            "required": []
        })
    }

    fn validate_exa_endpoint(&self) -> anyhow::Result<String> {
        validate_url(
            EXA_MCP_URL,
            &DomainPolicy {
                allowed_domains: &["mcp.exa.ai".to_string()],
                blocked_domains: &[],
                allowed_field_name: "codesearch.allowed_domains",
                blocked_field_name: None,
                empty_allowed_message: "codesearch endpoint allowlist is empty",
                scheme_policy: UrlSchemePolicy::HttpsOnly,
                ipv6_error_context: "codesearch",
            },
        )
    }

    fn resolve_query(args: &Args) -> String {
        args.query
            .as_deref()
            .or(args.information_request.as_deref())
            .map(str::trim)
            .unwrap_or_default()
            .to_string()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for CodeSearchTool {
    fn name(&self) -> &str {
        "codesearch"
    }

    fn description(&self) -> &str {
        include_str!("codesearch.txt")
    }

    fn parameters_schema(&self) -> serde_json::Value {
        Self::schema()
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(args)
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;

        let query = Self::resolve_query(&args);
        if query.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Missing query".to_string()),
            });
        }

        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            });
        }

        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        let endpoint = match self.validate_exa_endpoint() {
            Ok(url) => url,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(e.to_string()),
                });
            }
        };

        let tokens = args.tokens_num.unwrap_or(DEFAULT_TOKENS).clamp(MIN_TOKENS, MAX_TOKENS);
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "get_code_context_exa",
                "arguments": {
                    "query": query,
                    "tokensNum": tokens
                }
            }
        });

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(DEFAULT_TIMEOUT_MS))
            .build()?;

        let response = match client
            .post(&endpoint)
            .header(reqwest::header::ACCEPT, "application/json, text/event-stream")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&req)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(if e.is_timeout() {
                        "代码检索请求超时".to_string()
                    } else {
                        e.to_string()
                    }),
                });
            }
        };

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("代码检索出错（{}）：{}", status.as_u16(), body)),
            });
        }

        let output = parse_sse_first_text(&body).unwrap_or_else(|| {
            "未找到相关代码片段或文档。请尝试更换查询语句、明确库/编程概念名称，或检查框架名称拼写。"
                .to_string()
        });

        Ok(ToolResult { success: true, output, error: None })
    }
}

pub(crate) fn parse_sse_first_text(response_text: &str) -> Option<String> {
    for line in response_text.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("data: ") else {
            continue;
        };
        let v: serde_json::Value = serde_json::from_str(rest).ok()?;
        let text = v
            .get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())?;
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    None
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
