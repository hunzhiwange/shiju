//! MCP 远端工具规格映射。
//!
//! 本模块负责把 MCP `tools/list` 返回的工具定义转换为本地 [`ToolSpec`]，并提供单个
//! 动态 MCP 工具的执行路径。工具集注册和策略应用由后续批次接入。

use super::mcp_common::{
    find_server_by_name, load_resolved_acp_config, server_name, with_server_peer,
};
use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use agent_client_protocol::McpServer;
use anyhow::Context;
use async_trait::async_trait;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Content, JsonObject, RawContent,
    Tool as McpToolDefinition,
};
use rmcp::service::{Peer, RoleClient};
use serde_json::Value;
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
    sync::Arc,
};
use vw_api_types::tools::ToolResultContentDto;

const MCP_TOOL_ID_PREFIX: &str = "mcp";
const DESCRIPTION_INSTRUCTION_LIMIT: usize = 300;
const DESCRIPTION_LIMIT: usize = 1200;

type McpToolCallFuture =
    Pin<Box<dyn Future<Output = anyhow::Result<CallToolResult>> + Send + 'static>>;
type McpToolCallHandler = Arc<dyn Fn(String, JsonObject) -> McpToolCallFuture + Send + Sync>;

/// 单个动态 MCP 远端工具。
///
/// 实例持有发现阶段生成的本地规格，以及执行 `tools/call` 所需的 server/tool 稳定名称。
/// 默认执行器会从 ACP 配置查找 server，并在一次调用生命周期内通过 `with_server_peer`
/// 建立 peer。
#[derive(Clone)]
pub struct McpTool {
    server_name: String,
    tool_name: String,
    spec: ToolSpec,
    call_handler: McpToolCallHandler,
}

impl McpTool {
    /// 通过 MCP 工具定义创建动态工具。
    ///
    /// # 错误
    ///
    /// 当 server 或 tool 名称无法规范化成本地工具 ID 时返回错误。
    pub fn new(
        server_name: impl Into<String>,
        tool: McpToolDefinition,
        server_instructions: Option<&str>,
    ) -> anyhow::Result<Self> {
        let server_name = server_name.into();
        let tool_name = tool.name.to_string();
        let spec = mcp_tools_to_specs(&server_name, server_instructions, &[tool], &[])?
            .into_iter()
            .next()
            .context("MCP tool spec conversion returned no specs")?;
        Ok(Self::from_spec(server_name, tool_name, spec))
    }

    /// 通过已发现的本地规格创建动态工具。
    pub fn from_spec(
        server_name: impl Into<String>,
        tool_name: impl Into<String>,
        spec: ToolSpec,
    ) -> Self {
        let server_name = server_name.into();
        Self::with_call_fn(server_name.clone(), tool_name, spec, default_call_handler(server_name))
    }

    /// 通过已解析 server 配置创建动态工具。
    ///
    /// 工具执行时复用发现阶段的 server 定义，不再依赖进程当前目录重新查找配置。
    pub fn from_configured_server(
        server: McpServer,
        tool_name: impl Into<String>,
        spec: ToolSpec,
    ) -> Self {
        let server_name = server_name(&server).to_string();
        Self::with_call_fn(server_name, tool_name, spec, configured_call_handler(server))
    }

    #[cfg(test)]
    pub(crate) fn with_call_handler<F, Fut>(
        server_name: impl Into<String>,
        tool_name: impl Into<String>,
        spec: ToolSpec,
        handler: F,
    ) -> Self
    where
        F: Fn(String, JsonObject) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<CallToolResult>> + Send + 'static,
    {
        Self::with_call_fn(
            server_name,
            tool_name,
            spec,
            Arc::new(move |tool_name, arguments| Box::pin(handler(tool_name, arguments))),
        )
    }

    fn with_call_fn(
        server_name: impl Into<String>,
        tool_name: impl Into<String>,
        spec: ToolSpec,
        call_handler: McpToolCallHandler,
    ) -> Self {
        Self { server_name: server_name.into(), tool_name: tool_name.into(), spec, call_handler }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for McpTool {
    fn name(&self) -> &str {
        self.spec.id.as_str()
    }

    fn description(&self) -> &str {
        self.spec.description.as_str()
    }

    fn parameters_schema(&self) -> Value {
        self.spec.input_schema.clone()
    }

    fn spec(&self) -> ToolSpec {
        self.spec.clone()
    }

    fn is_read_only(&self) -> bool {
        self.spec.read_only
    }

    fn is_concurrency_safe(&self) -> bool {
        self.spec.concurrency_safe
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let result = self.call(args).await?;
        Ok(ToolResult {
            success: result.is_success(),
            output: result.model_text(),
            error: result.error_text(),
        })
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let arguments = input_as_json_object(input)?;
        let result =
            (self.call_handler)(self.tool_name.clone(), arguments).await.with_context(|| {
                format!("failed to call MCP tool '{}::{}'", self.server_name, self.tool_name)
            })?;
        Ok(mcp_call_result_to_tool_call_result(&self.server_name, &self.tool_name, result))
    }

    fn render_hint(&self, result: &ToolCallResult) -> Option<ToolRenderHint> {
        result.render_hint.clone().or_else(|| {
            Some(ToolRenderHint {
                title: Some(self.spec.display_name.clone()),
                kind: Some("mcp_tool".to_string()),
                summary: result.error_text(),
                metadata: serde_json::json!({
                    "server": self.server_name,
                    "tool": self.tool_name,
                    "tool_id": self.spec.id,
                }),
            })
        })
    }
}

/// 将 MCP server 和 tool 名称构造成稳定动态工具 ID。
///
/// # 错误
///
/// 当 server 或 tool 名称无法规范化为非空 ID 片段时返回错误。
pub fn mcp_tool_id(server_name: &str, tool_name: &str) -> anyhow::Result<String> {
    let server = normalize_mcp_name_segment(server_name)
        .with_context(|| format!("invalid MCP server name '{server_name}'"))?;
    let tool = normalize_mcp_name_segment(tool_name)
        .with_context(|| format!("invalid MCP tool name '{tool_name}'"))?;
    Ok(format!("{MCP_TOOL_ID_PREFIX}__{server}__{tool}"))
}

/// 发现指定 peer 的远端工具并转换为本地工具规格。
///
/// # 错误
///
/// 当 `tools/list` 请求失败，或返回工具名称与已有工具 ID 冲突时返回错误。
pub async fn discover_mcp_tool_specs(
    peer: &Peer<RoleClient>,
    server_name: &str,
    server_instructions: Option<&str>,
    existing_tool_ids: &[&str],
) -> anyhow::Result<Vec<ToolSpec>> {
    let tools = peer.list_all_tools().await.context("failed to list MCP tools")?;
    mcp_tools_to_specs(server_name, server_instructions, &tools, existing_tool_ids)
}

/// 将 MCP `tools/list` 返回的工具定义转换为本地工具规格。
///
/// `existing_tool_ids` 用于防止动态 MCP 工具覆盖内置工具或已注册工具。
///
/// # 错误
///
/// 当 server/tool 名称无法规范化，多个 MCP 工具规范化后重名，或与已有工具 ID 冲突时
/// 返回错误。
pub fn mcp_tools_to_specs(
    server_name: &str,
    server_instructions: Option<&str>,
    tools: &[McpToolDefinition],
    existing_tool_ids: &[&str],
) -> anyhow::Result<Vec<ToolSpec>> {
    let reserved_ids = existing_tool_ids.iter().copied().collect::<BTreeSet<_>>();
    let mut seen_ids = BTreeSet::new();
    let mut specs = Vec::with_capacity(tools.len());

    for tool in tools {
        let id = mcp_tool_id(server_name, tool.name.as_ref())?;
        if reserved_ids.contains(id.as_str()) {
            anyhow::bail!("MCP tool id '{id}' collides with an existing tool");
        }
        if !seen_ids.insert(id.clone()) {
            anyhow::bail!("duplicate MCP tool id '{id}'");
        }

        let input_schema = Value::Object(tool.input_schema.as_ref().clone());
        let description = build_description(tool.description.as_deref(), server_instructions);
        let display_name = format!("{server_name}: {}", display_tool_name(tool));
        let annotations = tool.annotations.as_ref();
        let read_only = annotations.and_then(|value| value.read_only_hint).unwrap_or(false);
        let destructive = if read_only {
            false
        } else {
            annotations.and_then(|value| value.destructive_hint).unwrap_or(true)
        };
        let concurrency_safe = annotations.is_some_and(|value| {
            value.read_only_hint == Some(true) || value.idempotent_hint == Some(true)
        });

        specs.push(
            ToolSpec::new(id, description, input_schema)
                .with_display_name(display_name)
                .with_read_only(read_only)
                .with_destructive(destructive)
                .with_concurrency_safe(concurrency_safe)
                .with_requires_user_interaction(false)
                .with_strict(true),
        );
    }

    Ok(specs)
}

fn default_call_handler(server_name: String) -> McpToolCallHandler {
    Arc::new(move |tool_name, arguments| {
        let server_name = server_name.clone();
        Box::pin(async move { call_remote_mcp_tool(&server_name, &tool_name, arguments).await })
    })
}

fn configured_call_handler(server: McpServer) -> McpToolCallHandler {
    Arc::new(move |tool_name, arguments| {
        let server = server.clone();
        Box::pin(
            async move { call_configured_remote_mcp_tool(&server, &tool_name, arguments).await },
        )
    })
}

async fn call_remote_mcp_tool(
    server_name: &str,
    tool_name: &str,
    arguments: JsonObject,
) -> anyhow::Result<CallToolResult> {
    let config = load_resolved_acp_config().await?;
    let server = find_server_by_name(&config.mcp_servers, server_name)?;

    with_server_peer(server, |peer: &Peer<RoleClient>| {
        let tool_name = tool_name.to_string();
        let arguments = arguments.clone();
        Box::pin(async move {
            let result = peer
                .call_tool(CallToolRequestParam {
                    name: Cow::Owned(tool_name),
                    arguments: Some(arguments),
                })
                .await?;
            Ok(result)
        })
    })
    .await
}

async fn call_configured_remote_mcp_tool(
    server: &McpServer,
    tool_name: &str,
    arguments: JsonObject,
) -> anyhow::Result<CallToolResult> {
    with_server_peer(server, |peer: &Peer<RoleClient>| {
        let tool_name = tool_name.to_string();
        let arguments = arguments.clone();
        Box::pin(async move {
            let result = peer
                .call_tool(CallToolRequestParam {
                    name: Cow::Owned(tool_name),
                    arguments: Some(arguments),
                })
                .await?;
            Ok(result)
        })
    })
    .await
}

fn input_as_json_object(input: Value) -> anyhow::Result<JsonObject> {
    match input {
        Value::Object(arguments) => Ok(arguments),
        _ => anyhow::bail!("MCP tool input must be a JSON object"),
    }
}

fn mcp_call_result_to_tool_call_result(
    server_name: &str,
    tool_name: &str,
    result: CallToolResult,
) -> ToolCallResult {
    let success = !result.is_error.unwrap_or(false);
    let content_json = result.content.iter().map(mcp_content_json).collect::<Vec<_>>();
    let content_blocks = result.content.iter().map(mcp_content_block).collect::<Vec<_>>();
    let model_result =
        mcp_model_result(&result.content, &content_json, result.structured_content.as_ref());
    let error_text = (!success).then(|| result_error_text(&model_result));

    let mut data = serde_json::json!({
        "success": success,
        "server": server_name,
        "tool": tool_name,
        "is_error": result.is_error.unwrap_or(false),
        "content": content_json,
        "structured_content": result.structured_content.unwrap_or(Value::Null),
    });
    if let Some(error) = &error_text {
        data["error"] = Value::String(error.clone());
    }

    let mut attributes = BTreeMap::new();
    attributes.insert("server".to_string(), Value::String(server_name.to_string()));
    attributes.insert("tool".to_string(), Value::String(tool_name.to_string()));
    attributes.insert(
        "content_count".to_string(),
        Value::Number(serde_json::Number::from(content_blocks.len())),
    );
    attributes.insert("is_error".to_string(), Value::Bool(!success));

    ToolCallResult {
        data,
        model_result,
        content_blocks,
        telemetry: Some(ToolCallTelemetry { success, attributes, ..ToolCallTelemetry::default() }),
        ..ToolCallResult::default()
    }
}

fn mcp_content_block(content: &Content) -> ToolResultContentDto {
    match &content.raw {
        RawContent::Text(text) => ToolResultContentDto::Text { text: text.text.clone() },
        _ => ToolResultContentDto::Json { value: mcp_content_json(content) },
    }
}

fn mcp_content_json(content: &Content) -> Value {
    serde_json::to_value(content).unwrap_or_else(|_| {
        serde_json::json!({
            "type": "unknown",
        })
    })
}

fn mcp_model_result(
    contents: &[Content],
    content_json: &[Value],
    structured_content: Option<&Value>,
) -> Value {
    let text = contents
        .iter()
        .filter_map(|content| match &content.raw {
            RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !text.is_empty() {
        return Value::String(text);
    }

    if let Some(structured_content) = structured_content {
        return structured_content.clone();
    }

    Value::Array(content_json.to_vec())
}

fn result_error_text(model_result: &Value) -> String {
    match model_result {
        Value::String(text) => text.clone(),
        other => serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
    }
}

fn normalize_mcp_name_segment(raw: &str) -> anyhow::Result<String> {
    let mut normalized = String::new();
    let mut previous_was_separator = false;

    for character in raw.trim().chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
        }
    }

    let normalized = normalized.trim_matches('_').to_string();
    if normalized.is_empty() {
        anyhow::bail!("normalized name segment must not be empty");
    }
    Ok(normalized)
}

fn display_tool_name(tool: &McpToolDefinition) -> String {
    tool.title
        .as_ref()
        .or_else(|| tool.annotations.as_ref().and_then(|value| value.title.as_ref()))
        .cloned()
        .unwrap_or_else(|| tool.name.to_string())
}

fn build_description(tool_description: Option<&str>, server_instructions: Option<&str>) -> String {
    let mut parts = Vec::new();
    if let Some(description) = non_empty_normalized(tool_description) {
        parts.push(description);
    }
    if let Some(summary) = server_instructions.and_then(summarize_server_instructions) {
        parts.push(format!("Server instructions: {summary}"));
    }

    let description =
        if parts.is_empty() { "Remote MCP tool.".to_string() } else { parts.join("\n\n") };
    truncate_with_suffix(&description, DESCRIPTION_LIMIT)
}

fn summarize_server_instructions(instructions: &str) -> Option<String> {
    let sanitized =
        instructions.lines().filter(|line| !looks_sensitive(line)).collect::<Vec<_>>().join(" ");
    non_empty_normalized(Some(&sanitized))
        .map(|value| truncate_with_suffix(&value, DESCRIPTION_INSTRUCTION_LIMIT))
}

fn non_empty_normalized(value: Option<&str>) -> Option<String> {
    let normalized = value?.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() { None } else { Some(normalized) }
}

fn looks_sensitive(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    [
        "authorization",
        "bearer ",
        "api_key",
        "api-key",
        "x-api-key",
        "token",
        "secret",
        "password",
        "set-cookie",
        "cookie:",
    ]
    .iter()
    .any(|needle| line.contains(needle))
}

fn truncate_with_suffix(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut truncated = value.chars().take(max_chars.saturating_sub(3)).collect::<String>();
    truncated.push_str("...");
    truncated
}
