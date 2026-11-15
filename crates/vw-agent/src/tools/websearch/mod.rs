//! # 网络搜索工具（Exa AI）
//!
//! 本模块提供基于 Exa AI 搜索引擎的网络搜索能力，作为代理工具集成到 VibeWindow 运行时中。
//!
//! ## 功能概述
//!
//! - 使用 Exa AI 的 MCP（Model Context Protocol）端点执行网络搜索
//! - 支持多种搜索模式：auto（自动）、fast（快速）、deep（深度）
//! - 支持实时抓取配置：fallback（回退）、preferred（优先）
//! - 返回结构化的搜索结果，包含上下文文本内容
//! - 内置安全策略检查，包括操作权限、速率限制和 URL 验证
//!
//! ## 架构设计
//!
//! - 实现 [`Tool`] trait，符合工具扩展点的标准契约
//! - 通过安全策略 [`SecurityPolicy`] 进行访问控制和速率限制
//! - 使用 URL 验证机制确保只访问允许的域名端点
//! - 支持 WASM 和原生平台，在 WASM 上不设置超时（受平台限制）
//!
//! ## 使用示例
//!
//! ```ignore
//! use std::sync::Arc;
//! use crate::app::agent::security::SecurityPolicy;
//! use crate::app::agent::tools::websearch::WebSearchTool;
//! use crate::app::agent::tools::traits::Tool;
//!
//! let security = Arc::new(SecurityPolicy::default());
//! let tool = WebSearchTool::new(
//!     security,
//!     "exa".to_string(),
//!     None,
//!     None,
//!     5,
//!     30,
//!     "VibeWindowAgent".to_string(),
//! );
//!
//! let args = serde_json::json!({
//!     "query": "Rust programming language",
//!     "numResults": 5,
//!     "type": "auto",
//!     "livecrawl": "fallback",
//! });
//!
//! let result = tool.execute(args).await?;
//! println!("Search results: {}", result.output);
//! ```

use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use super::url_validation::{DomainPolicy, UrlSchemePolicy, validate_url};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use vw_api_tools::ToolResultContentDto;

/// Exa AI MCP 服务端点的 URL 地址
///
/// 该端点用于通过 MCP（Model Context Protocol）协议与 Exa AI 搜索服务通信。
/// 所有搜索请求都发送到此端点。
const EXA_MCP_URL: &str = "https://mcp.exa.ai/mcp";

/// 默认的请求超时时间（毫秒）
///
/// 当用户未指定超时时间或指定为 0 时，使用此默认值。
/// 设置为 25 秒以平衡响应速度和网络延迟容忍度。
const DEFAULT_TIMEOUT_MS: u64 = 25_000;

/// WebSearch 工具的参数结构体
///
/// 该结构体用于反序列化工具调用时传入的 JSON 参数，
/// 包含搜索查询和可选的搜索配置选项。
///
/// # 字段说明
///
/// * `query` - 搜索查询字符串（必填），为空时将返回错误
/// * `num_results` - 返回结果数量（可选），默认由工具配置决定，最大 10 条
/// * `livecrawl` - 实时抓取策略（可选），可选值：
///   - `"fallback"`: 仅在必要时实时抓取（默认）
///   - `"preferred"`: 优先实时抓取最新内容
/// * `search_type` - 搜索类型（可选），可选值：
///   - `"auto"`: 自动选择最佳模式（默认）
///   - `"fast"`: 快速模式，适合简单查询
///   - `"deep"`: 深度模式，适合复杂研究
/// * `context_max_characters` - 单个结果的上下文最大字符数（可选）
#[derive(Debug, Clone, Deserialize)]
struct Args {
    /// 搜索查询字符串，必填字段
    query: Option<String>,
    /// 返回结果数量，可选字段，默认值由工具配置决定
    #[serde(
        rename = "numResults",
        default,
        alias = "num",
        alias = "num_results",
        alias = "max_results"
    )]
    num_results: Option<u32>,
    /// 实时抓取策略："fallback" 或 "preferred"
    livecrawl: Option<String>,
    /// 搜索类型："auto"、"fast" 或 "deep"
    #[serde(rename = "type")]
    search_type: Option<String>,
    /// 单个结果的上下文最大字符数
    #[serde(rename = "contextMaxCharacters")]
    context_max_characters: Option<u32>,
    /// Claude 风格语言限制提示，当前先兼容输入表面。
    #[serde(default, alias = "language", alias = "lang")]
    lr: Option<String>,
}

/// 网络搜索工具
///
/// 基于 Exa AI 搜索引擎的工具实现，提供高质量的网络搜索能力。
/// 该工具通过 MCP 协议与 Exa AI 通信，返回结构化的搜索结果。
///
/// # 特性
///
/// - **安全检查**: 执行前验证操作权限和速率限制
/// - **URL 验证**: 确保只访问允许的域名端点
/// - **参数校验**: 验证所有输入参数的合法性
/// - **错误处理**: 返回详细的错误信息，不使用 panic
/// - **跨平台**: 支持 WASM 和原生平台
///
/// # 示例
///
/// ```ignore
/// let tool = WebSearchTool::new(security, provider, api_key, api_url, 5, 30, user_agent);
/// let result = tool.execute(json!({"query": "hello world"})).await?;
/// ```
pub struct WebSearchTool {
    /// 安全策略引用，用于权限检查和速率限制
    security: Arc<SecurityPolicy>,
    /// 默认返回结果数量，从配置中提取并限制在 [1, 10] 范围内
    default_num_results: u32,
    /// 请求超时时间（毫秒），0 表示使用默认值
    timeout_ms: u64,
}

impl WebSearchTool {
    /// 创建新的 WebSearchTool 实例
    ///
    /// # 参数
    ///
    /// * `security` - 安全策略引用，用于操作权限和速率限制检查
    /// * `_provider` - 提供商标识符（当前未使用，保留用于未来扩展）
    /// * `_api_key` - API 密钥（当前未使用，Exa AI 通过 MCP 端点认证）
    /// * `_api_url` - 自定义 API URL（当前未使用，使用固定的 EXA_MCP_URL）
    /// * `max_results` - 最大返回结果数，将被限制在 [1, 10] 范围内
    /// * `timeout_secs` - 请求超时时间（秒），0 表示使用默认值（25 秒）
    /// * `_user_agent` - 用户代理字符串（当前未使用，保留用于未来扩展）
    ///
    /// # 返回值
    ///
    /// 返回配置好的 `WebSearchTool` 实例
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let tool = WebSearchTool::new(
    ///     security,
    ///     "exa".to_string(),
    ///     None,
    ///     None,
    ///     5,
    ///     30,
    ///     "VibeWindowAgent".to_string(),
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        security: Arc<SecurityPolicy>,
        _provider: String,
        _api_key: Option<String>,
        _api_url: Option<String>,
        max_results: usize,
        timeout_secs: u64,
        _user_agent: String,
    ) -> Self {
        // 限制默认结果数量在合理范围内，避免过多结果影响性能
        let default_num_results = u32::try_from(max_results.clamp(1, 10)).unwrap_or(10);
        // 转换超时时间为毫秒，0 表示使用默认值
        let timeout_ms = if timeout_secs == 0 { DEFAULT_TIMEOUT_MS } else { timeout_secs * 1000 };
        Self { security, default_num_results, timeout_ms }
    }

    /// 生成工具参数的 JSON Schema
    ///
    /// 返回符合 JSON Schema 规范的参数定义，用于工具描述和参数验证。
    /// 该 schema 定义了所有可接受的参数及其类型和可选值。
    ///
    /// # 返回值
    ///
    /// 返回包含参数定义的 JSON 对象
    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                // 搜索查询字符串，必填
                "query": { "type": "string" },
                // Claude 风格结果数兼容字段
                "num": { "type": "number" },
                // 返回结果数量，数字类型
                "numResults": { "type": "number" },
                // Claude 风格语言限制兼容字段
                "lr": { "type": "string" },
                // 实时抓取策略，枚举值
                "livecrawl": { "type": "string", "enum": ["fallback", "preferred"] },
                // 搜索类型，枚举值
                "type": { "type": "string", "enum": ["auto", "fast", "deep"] },
                // 上下文最大字符数
                "contextMaxCharacters": { "type": "number" }
            },
            // query 为必填字段
            "required": ["query"]
        })
    }

    /// 验证 Exa AI 端点 URL
    ///
    /// 使用 URL 验证机制确保只访问允许的域名（mcp.exa.ai），
    /// 防止 SSRF（服务器端请求伪造）攻击。
    ///
    /// # 返回值
    ///
    /// - `Ok(String)` - 验证通过的 URL 字符串
    /// - `Err(anyhow::Error)` - URL 验证失败，包含详细的错误信息
    ///
    /// # 安全策略
    ///
    /// - 仅允许 HTTPS 协议
    /// - 仅允许访问 mcp.exa.ai 域名
    /// - 禁止 IPv6 地址访问（避免绕过域名限制）
    fn validate_exa_endpoint(&self) -> anyhow::Result<String> {
        validate_url(
            EXA_MCP_URL,
            &DomainPolicy {
                // 仅允许 Exa AI MCP 域名
                allowed_domains: &["mcp.exa.ai".to_string()],
                // 无黑名单域名
                blocked_domains: &[],
                // 配置字段名，用于错误信息
                allowed_field_name: "websearch.allowed_domains",
                blocked_field_name: None,
                // 空允许列表时的错误信息
                empty_allowed_message: "websearch endpoint allowlist is empty",
                // 仅允许 HTTPS 协议
                scheme_policy: UrlSchemePolicy::HttpsOnly,
                // IPv6 错误上下文
                ipv6_error_context: "websearch",
            },
        )
    }

    /// 解析 SSE（Server-Sent Events）响应中的首个文本结果
    ///
    /// Exa AI 返回的响应采用 SSE 格式，每行以 "data: " 前缀开头。
    /// 该方法提取第一个包含非空文本的搜索结果。
    ///
    /// # 参数
    ///
    /// * `response_text` - SSE 格式的原始响应文本
    ///
    /// # 返回值
    ///
    /// - `Some(String)` - 成功提取的文本内容
    /// - `None` - 无有效文本结果或解析失败
    ///
    /// # SSE 格式说明
    ///
    /// 响应格式示例：
    /// ```text
    /// data: {"result": {"content": [{"text": "搜索结果文本..."}]}}
    /// ```
    ///
    /// # 提取路径
    ///
    /// `result -> content[0] -> text`
    pub(crate) fn parse_sse_first_text(response_text: &str) -> Option<String> {
        // 逐行处理 SSE 响应
        for line in response_text.lines() {
            let line = line.trim();
            // 查找 "data: " 前缀
            let Some(rest) = line.strip_prefix("data: ") else {
                continue;
            };

            // 解析 JSON 数据
            let Ok(value) = serde_json::from_str::<serde_json::Value>(rest) else {
                continue;
            };

            // 按路径提取文本：result -> content -> array[0] -> text
            let Some(text) = value
                .get("result")
                .and_then(|r| r.get("content"))
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("text"))
                .and_then(|t| t.as_str())
            else {
                continue;
            };

            // 只返回非空文本
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
        None
    }

    fn parse_numbered_results(output: &str) -> Vec<Value> {
        let mut lines = output.lines();
        let Some(first_line) = lines.next().map(str::trim) else {
            return Vec::new();
        };
        if !first_line.starts_with("Search results for:") {
            return Vec::new();
        }

        let mut results = Vec::<Value>::new();
        let mut current_title: Option<String> = None;
        let mut current_url: Option<String> = None;
        let mut current_snippet: Option<String> = None;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some((index, title)) = trimmed.split_once('.')
                && index.parse::<usize>().is_ok()
                && !title.trim().is_empty()
            {
                if let Some(title) = current_title.take() {
                    results.push(json!({
                        "title": title,
                        "url": current_url.take(),
                        "snippet": current_snippet.take(),
                    }));
                }
                current_title = Some(title.trim().to_string());
                continue;
            }

            if current_title.is_none() {
                continue;
            }

            if current_url.is_none()
                && (trimmed.starts_with("http://") || trimmed.starts_with("https://"))
            {
                current_url = Some(trimmed.to_string());
                continue;
            }

            current_snippet = Some(match current_snippet.take() {
                Some(existing) if !existing.is_empty() => format!("{existing} {trimmed}"),
                _ => trimmed.to_string(),
            });
        }

        if let Some(title) = current_title {
            results.push(json!({
                "title": title,
                "url": current_url,
                "snippet": current_snippet,
            }));
        }

        results
    }
}

/// Tool trait 实现
///
/// 为 WebSearchTool 实现 Tool trait，使其能够作为代理工具被调用。
/// 支持跨平台异步执行（WASM 和原生平台）。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for WebSearchTool {
    /// 返回工具名称
    ///
    /// 该名称用于工具注册和调用时的标识符。
    ///
    /// # 返回值
    ///
    /// 固定返回 `"web_search_tool"`
    fn name(&self) -> &str {
        "web_search_tool"
    }

    /// 返回工具描述
    ///
    /// 从外部文件 `websearch.txt` 加载工具的详细描述信息，
    /// 用于向用户或代理展示工具的用途和使用方法。
    ///
    /// # 返回值
    ///
    /// 工具描述文本
    fn description(&self) -> &str {
        include_str!("websearch.txt")
    }

    /// 返回工具参数的 JSON Schema
    ///
    /// 定义工具接受的所有参数及其类型约束，
    /// 用于参数验证和工具文档生成。
    ///
    /// # 返回值
    ///
    /// 符合 JSON Schema 规范的参数定义
    fn parameters_schema(&self) -> serde_json::Value {
        Self::schema()
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(
            crate::app::agent::tools::WEB_SEARCH_TOOL_ID,
            self.description(),
            self.parameters_schema(),
        )
        .with_display_name(crate::app::agent::tools::WEB_SEARCH_TOOL_ID)
        .with_aliases(vec![
            crate::app::agent::tools::WEB_SEARCH_TOOL_ALIAS.to_string(),
            "websearch".to_string(),
            "web_search".to_string(),
        ])
        .with_read_only(true)
        .with_destructive(false)
        .with_concurrency_safe(true)
        .with_requires_user_interaction(false)
        .with_strict(true)
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let legacy = self.execute(input.clone()).await?;
        let parsed_args = serde_json::from_value::<Args>(input).ok();
        let query = parsed_args
            .as_ref()
            .and_then(|args| args.query.clone())
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        let requested_lr =
            parsed_args.as_ref().and_then(|args| args.lr.clone()).unwrap_or_default();

        if !legacy.success {
            let mut result = ToolCallResult::from_legacy_result(legacy);
            result.render_hint = Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::WEB_SEARCH_TOOL_ID.to_string()),
                kind: Some("web_search".to_string()),
                summary: Some(if query.is_empty() {
                    "Web search failed".to_string()
                } else {
                    format!("Web search failed for {query}")
                }),
                metadata: json!({
                    "provider": "Exa",
                    "query": query,
                    "lr": requested_lr,
                }),
            });
            return Ok(result);
        }

        let raw = legacy.output;
        let results = Self::parse_numbered_results(&raw);
        let result_count = results.len();
        let data = json!({
            "query": query.clone(),
            "provider": "Exa",
            "lr": requested_lr.clone(),
            "results": results.clone(),
            "raw": raw.clone(),
        });

        Ok(ToolCallResult {
            data,
            model_result: Value::String(raw),
            content_blocks: vec![ToolResultContentDto::Json {
                value: json!({
                    "query": query.clone(),
                    "provider": "Exa",
                    "result_count": result_count,
                    "results": results,
                }),
            }],
            render_hint: Some(ToolRenderHint {
                title: Some(crate::app::agent::tools::WEB_SEARCH_TOOL_ID.to_string()),
                kind: Some("web_search".to_string()),
                summary: Some(if result_count == 0 {
                    if query.is_empty() {
                        "Web search completed".to_string()
                    } else {
                        format!("Web search completed for {query}")
                    }
                } else if query.is_empty() {
                    format!("Found {result_count} results")
                } else {
                    format!("Found {result_count} results for {query}")
                }),
                metadata: json!({
                    "provider": "Exa",
                    "lr": requested_lr,
                    "result_count": result_count,
                }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        })
    }

    /// 执行网络搜索
    ///
    /// 该方法实现完整的搜索流程，包括：
    /// 1. 参数解析和验证
    /// 2. 安全策略检查（操作权限、速率限制）
    /// 3. 端点 URL 验证
    /// 4. 构建并发送 MCP 请求
    /// 5. 解析 SSE 响应并提取结果
    ///
    /// # 参数
    ///
    /// * `args` - JSON 格式的工具参数，必须包含 `query` 字段
    ///
    /// # 返回值
    ///
    /// 返回 `ToolResult` 结构体：
    /// - `success: true, output: "..."` - 搜索成功，包含结果文本
    /// - `success: false, error: Some("...")` - 搜索失败，包含错误信息
    ///
    /// # 错误情况
    ///
    /// - 缺少或无效的参数
    /// - 查询字符串为空
    /// - 参数值不在允许范围内
    /// - 安全策略阻止操作（只读模式）
    /// - 速率限制触发
    /// - URL 验证失败
    /// - 网络请求超时或失败
    /// - 服务端返回错误状态码
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let result = tool.execute(json!({
    ///     "query": "Rust async programming",
    ///     "numResults": 5,
    ///     "type": "deep",
    /// })).await?;
    ///
    /// if result.success {
    ///     println!("Results: {}", result.output);
    /// } else {
    ///     eprintln!("Error: {:?}", result.error);
    /// }
    /// ```
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 1. 解析并验证参数
        let args: Args = serde_json::from_value(args)
            .map_err(|e| anyhow::anyhow!("Missing or invalid parameters: {e}"))?;

        // 提取并清理查询字符串
        let query = args.query.as_deref().map(str::trim).unwrap_or_default().to_string();
        if query.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("query cannot be empty".to_string()),
            });
        }

        // 验证 livecrawl 参数
        let livecrawl = args.livecrawl.as_deref().unwrap_or("fallback");
        if !matches!(livecrawl, "fallback" | "preferred") {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("livecrawl must be 'fallback' or 'preferred'".to_string()),
            });
        }

        // 验证 search_type 参数
        let search_type = args.search_type.as_deref().unwrap_or("auto");
        if !matches!(search_type, "auto" | "fast" | "deep") {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("type must be 'auto', 'fast', or 'deep'".to_string()),
            });
        }

        // 2. 安全策略检查：验证是否允许执行操作
        if !self.security.can_act() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        // 检查速率限制
        if self.security.is_rate_limited() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: too many actions in the last hour".into()),
            });
        }

        // 记录操作并检查配额
        if !self.security.record_action() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Rate limit exceeded: action budget exhausted".into()),
            });
        }

        // 3. 验证端点 URL 安全性
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

        // 4. 构建 MCP 请求体
        let num_results = args.num_results.unwrap_or(self.default_num_results);
        let request_body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "web_search_exa",
                "arguments": {
                    "query": query,
                    "type": search_type,
                    "numResults": num_results,
                    "livecrawl": livecrawl,
                    "contextMaxCharacters": args.context_max_characters,
                }
            }
        });

        // 5. 构建 HTTP 客户端
        let builder = reqwest::Client::builder();
        // 仅在非 WASM 平台设置超时（WASM 平台不支持）
        #[cfg(not(target_arch = "wasm32"))]
        let builder = builder.timeout(Duration::from_millis(self.timeout_ms));
        let client = builder.build()?;

        // 6. 发送搜索请求
        let response = match client
            .post(&endpoint)
            .header(reqwest::header::ACCEPT, "application/json, text/event-stream")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&request_body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                // 检查是否为超时错误（平台相关）
                let is_timeout = {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        e.is_timeout()
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        false
                    }
                };
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(if is_timeout {
                        "search request timed out".to_string()
                    } else {
                        e.to_string()
                    }),
                });
            }
        };

        // 7. 处理响应
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        // 检查 HTTP 状态码
        if !status.is_success() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("search error ({}): {}", status.as_u16(), body)),
            });
        }

        // 8. 解析 SSE 响应并提取搜索结果
        let output = Self::parse_sse_first_text(&body)
            .unwrap_or_else(|| "No search results found. Try refining your query.".to_string());

        Ok(ToolResult { success: true, output, error: None })
    }
}

/// 单元测试模块
///
/// 测试文件位于 `tests/websearch.rs`，包含 WebSearchTool 的各种测试用例。
#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
