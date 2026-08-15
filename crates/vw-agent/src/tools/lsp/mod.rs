//! LSP（语言服务器协议）工具。
//!
//! 对齐 Claude Code 的工具名与操作集合，在本地直接启动语言服务器进程，
//! 通过 stdio 与其交互，并返回结构化结果与专用渲染提示。

mod backend;
mod config;
mod format;
use self::backend::LspBackendSession;
use self::format::{LspPayload, format_operation_result};
use super::external_directory;
use super::traits::{
    Tool, ToolCallResult, ToolCallTelemetry, ToolRenderHint, ToolResult, ToolSpec,
};
use crate::app::agent::security::SecurityPolicy;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use vw_api_types::tools::ToolResultContentDto;

const LSP_TOOL_ID: &str = "lsp";
const MAX_LSP_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024;
const SUPPORTED_OPERATIONS: &[&str] = &[
    "goToDefinition",
    "findReferences",
    "hover",
    "documentSymbol",
    "workspaceSymbol",
    "goToImplementation",
    "prepareCallHierarchy",
    "incomingCalls",
    "outgoingCalls",
];

#[derive(Debug, Clone, Deserialize)]
struct Args {
    #[serde(default)]
    operation: Option<String>,
    #[serde(default, alias = "filePath", alias = "path")]
    file_path: Option<String>,
    #[serde(default)]
    uri: Option<String>,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    character: Option<u32>,
    #[serde(default)]
    query: Option<String>,
}

#[derive(Debug, Clone)]
struct ValidatedArgs {
    operation: String,
    requested_path: String,
    absolute_path: PathBuf,
    line: Option<u32>,
    character: Option<u32>,
    query: Option<String>,
}

pub struct LspTool {
    security: Arc<SecurityPolicy>,
}

impl LspTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }

    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": SUPPORTED_OPERATIONS,
                    "description": "要执行的 LSP 操作。"
                },
                "filePath": {
                    "type": "string",
                    "description": "目标文件路径。可为绝对路径或相对工作区路径。"
                },
                "path": {
                    "type": "string",
                    "description": "filePath 的兼容别名。"
                },
                "uri": {
                    "type": "string",
                    "description": "兼容旧调用面的 file URI。"
                },
                "line": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "基于 1 的行号。位置类操作必填。"
                },
                "character": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "基于 1 的字符偏移。位置类操作必填。"
                },
                "query": {
                    "type": "string",
                    "description": "workspaceSymbol 操作的可选查询文本；为空时返回全部工作区符号。"
                }
            }
        })
    }

    fn effective_file_path(args: &Args) -> Option<String> {
        args.file_path.clone().or_else(|| {
            args.uri
                .as_deref()
                .map(|uri| uri.strip_prefix("file://").unwrap_or(uri).replace("%20", " "))
        })
    }

    fn requires_position(operation: &str) -> bool {
        matches!(
            operation,
            "goToDefinition"
                | "findReferences"
                | "hover"
                | "goToImplementation"
                | "prepareCallHierarchy"
                | "incomingCalls"
                | "outgoingCalls"
        )
    }

    fn resolve_requested_path(&self, requested: &str) -> PathBuf {
        if Path::new(requested).is_absolute() {
            PathBuf::from(requested)
        } else {
            self.security.workspace_dir.join(requested)
        }
    }

    async fn validate_args(&self, args: &Args) -> anyhow::Result<ValidatedArgs> {
        let operation = args
            .operation
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Missing operation"))?;
        if !SUPPORTED_OPERATIONS.contains(&operation) {
            anyhow::bail!("Unsupported LSP operation: {operation}");
        }

        let requested_path = Self::effective_file_path(args)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Missing filePath"))?;

        if !self.security.is_path_allowed(&requested_path) {
            anyhow::bail!("Path not allowed by security policy: {requested_path}");
        }

        if Self::requires_position(operation) {
            let line = args.line.ok_or_else(|| anyhow::anyhow!("Missing line"))?;
            let character = args.character.ok_or_else(|| anyhow::anyhow!("Missing character"))?;
            if line == 0 || character == 0 {
                anyhow::bail!("line and character must be >= 1");
            }
        }

        let requested_full_path = self.resolve_requested_path(&requested_path);
        let requested_full_path_str = requested_full_path.to_string_lossy().to_string();
        external_directory::assert_external_directory(
            &self.security,
            Some(&requested_full_path_str),
            Some(external_directory::Options {
                bypass: false,
                kind: external_directory::Kind::File,
            }),
        )
        .await
        .map_err(anyhow::Error::msg)?;

        let absolute_path = fs::canonicalize(&requested_full_path)
            .await
            .unwrap_or_else(|_| requested_full_path.clone());
        let absolute_path_str = absolute_path.to_string_lossy().to_string();
        external_directory::assert_external_directory(
            &self.security,
            Some(&absolute_path_str),
            Some(external_directory::Options {
                bypass: false,
                kind: external_directory::Kind::File,
            }),
        )
        .await
        .map_err(anyhow::Error::msg)?;

        let metadata = fs::metadata(&absolute_path).await.map_err(|error| {
            anyhow::anyhow!("Cannot access file {}: {error}", absolute_path.display())
        })?;
        if !metadata.is_file() {
            anyhow::bail!("Path is not a file: {}", absolute_path.display());
        }

        Ok(ValidatedArgs {
            operation: operation.to_string(),
            requested_path,
            absolute_path,
            line: args.line,
            character: args.character,
            query: args.query.clone().map(|value| value.trim().to_string()),
        })
    }

    async fn execute_internal(&self, args: &ValidatedArgs) -> anyhow::Result<ToolCallResult> {
        if self.security.is_rate_limited() {
            anyhow::bail!("Rate limit exceeded: too many actions in the last hour");
        }
        if !self.security.record_action() {
            anyhow::bail!("Rate limit exceeded: action budget exhausted");
        }

        let metadata = fs::metadata(&args.absolute_path).await?;
        if metadata.len() > MAX_LSP_FILE_SIZE_BYTES {
            anyhow::bail!(
                "File too large for LSP analysis: {} exceeds {} MB",
                args.absolute_path.display(),
                MAX_LSP_FILE_SIZE_BYTES / 1_000_000
            );
        }

        let file_content = fs::read_to_string(&args.absolute_path).await.map_err(|error| {
            anyhow::anyhow!("Failed to read file {}: {error}", args.absolute_path.display())
        })?;

        let Some(session) = LspBackendSession::open(
            &self.security.workspace_dir,
            &args.absolute_path,
            &file_content,
        )
        .await?
        else {
            let extension =
                args.absolute_path.extension().and_then(|value| value.to_str()).unwrap_or("");
            return Ok(self.failure_result(
                args,
                &format!("No LSP server available for file type: .{extension}"),
                None,
            ));
        };

        let (method, params) = self.method_and_params(args, session.uri());
        let mut result = session.request(method, params).await?;
        if matches!(args.operation.as_str(), "incomingCalls" | "outgoingCalls") {
            let first_item =
                result.as_array().and_then(|items| items.first()).cloned().ok_or_else(|| {
                    anyhow::anyhow!("No call hierarchy item found at this position")
                })?;
            let followup_method = if args.operation == "incomingCalls" {
                "callHierarchy/incomingCalls"
            } else {
                "callHierarchy/outgoingCalls"
            };
            result = session.request(followup_method, json!({ "item": first_item })).await?;
        }

        let formatted =
            format_operation_result(&args.operation, &result, &self.security.workspace_dir);
        Ok(self.success_result(args, formatted, session.server_key(), session.language_id()))
    }

    fn success_result(
        &self,
        args: &ValidatedArgs,
        formatted: format::LspFormattedResponse,
        server_key: &str,
        language_id: &str,
    ) -> ToolCallResult {
        let payload_kind = match &formatted.payload {
            LspPayload::Locations { .. } => "locations",
            LspPayload::Hover { .. } => "hover",
            LspPayload::DocumentSymbols { .. } => "document_symbols",
            LspPayload::CallHierarchyItems { .. } => "call_hierarchy_items",
            LspPayload::CallHierarchyCalls { .. } => "call_hierarchy_calls",
            LspPayload::Message { .. } => "message",
        };
        let data = json!({
            "success": true,
            "implemented": true,
            "operation": args.operation,
            "file_path": args.requested_path,
            "absolute_path": args.absolute_path,
            "line": args.line,
            "character": args.character,
            "query": args.query,
            "result_text": formatted.result_text,
            "result_count": formatted.result_count,
            "file_count": formatted.file_count,
            "payload": formatted.payload,
            "server_key": server_key,
            "language_id": language_id,
        });

        ToolCallResult {
            data: data.clone(),
            model_result: Value::String(formatted.result_text.clone()),
            content_blocks: vec![
                ToolResultContentDto::Text { text: formatted.result_text.clone() },
                ToolResultContentDto::Json { value: data.clone() },
            ],
            render_hint: Some(ToolRenderHint {
                title: Some("LSP".to_string()),
                kind: Some("lsp".to_string()),
                summary: Some(formatted.summary.clone()),
                metadata: json!({
                    "tool_id": LSP_TOOL_ID,
                    "canonical_tool_id": LSP_TOOL_ID,
                    "implemented": true,
                    "success": true,
                    "operation": args.operation,
                    "result_count": formatted.result_count,
                    "file_count": formatted.file_count,
                    "path": args.requested_path,
                    "server_key": server_key,
                    "payload_kind": payload_kind,
                }),
            }),
            telemetry: Some(ToolCallTelemetry { success: true, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        }
    }

    fn failure_result(
        &self,
        args: &ValidatedArgs,
        message: &str,
        server_key: Option<&str>,
    ) -> ToolCallResult {
        let data = json!({
            "success": false,
            "implemented": true,
            "operation": args.operation,
            "file_path": args.requested_path,
            "absolute_path": args.absolute_path,
            "line": args.line,
            "character": args.character,
            "query": args.query,
            "result_text": message,
            "result_count": 0,
            "file_count": 0,
            "payload": { "kind": "message", "message": message },
            "error": message,
            "server_key": server_key,
        });

        ToolCallResult {
            data: data.clone(),
            model_result: Value::String(message.to_string()),
            content_blocks: vec![
                ToolResultContentDto::Text { text: message.to_string() },
                ToolResultContentDto::Json { value: data.clone() },
            ],
            render_hint: Some(ToolRenderHint {
                title: Some("LSP".to_string()),
                kind: Some("lsp".to_string()),
                summary: Some(message.to_string()),
                metadata: json!({
                    "tool_id": LSP_TOOL_ID,
                    "canonical_tool_id": LSP_TOOL_ID,
                    "implemented": true,
                    "success": false,
                    "operation": args.operation,
                    "path": args.requested_path,
                    "server_key": server_key,
                }),
            }),
            telemetry: Some(ToolCallTelemetry { success: false, ..ToolCallTelemetry::default() }),
            ..ToolCallResult::default()
        }
    }

    fn method_and_params<'a>(
        &self,
        args: &'a ValidatedArgs,
        uri: &'a str,
    ) -> (&'static str, Value) {
        let position = json!({
            "line": args.line.unwrap_or(1).saturating_sub(1),
            "character": args.character.unwrap_or(1).saturating_sub(1),
        });

        match args.operation.as_str() {
            "goToDefinition" => (
                "textDocument/definition",
                json!({ "textDocument": { "uri": uri }, "position": position }),
            ),
            "findReferences" => (
                "textDocument/references",
                json!({
                    "textDocument": { "uri": uri },
                    "position": position,
                    "context": { "includeDeclaration": true }
                }),
            ),
            "hover" => (
                "textDocument/hover",
                json!({ "textDocument": { "uri": uri }, "position": position }),
            ),
            "documentSymbol" => {
                ("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri } }))
            }
            "workspaceSymbol" => {
                ("workspace/symbol", json!({ "query": args.query.clone().unwrap_or_default() }))
            }
            "goToImplementation" => (
                "textDocument/implementation",
                json!({ "textDocument": { "uri": uri }, "position": position }),
            ),
            "prepareCallHierarchy" | "incomingCalls" | "outgoingCalls" => (
                "textDocument/prepareCallHierarchy",
                json!({ "textDocument": { "uri": uri }, "position": position }),
            ),
            _ => (
                "textDocument/hover",
                json!({ "textDocument": { "uri": uri }, "position": position }),
            ),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for LspTool {
    fn name(&self) -> &str {
        LSP_TOOL_ID
    }

    fn description(&self) -> &str {
        include_str!("lsp.txt")
    }

    fn parameters_schema(&self) -> serde_json::Value {
        Self::schema()
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(LSP_TOOL_ID, self.description(), self.parameters_schema())
            .with_display_name(LSP_TOOL_ID)
            .with_read_only(true)
            .with_destructive(false)
            .with_concurrency_safe(true)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    fn validate_input(&self, input: Value) -> anyhow::Result<Value> {
        if !input.is_object() {
            anyhow::bail!("Missing or invalid parameters: expected object");
        }
        let args: Args = serde_json::from_value(input.clone())
            .map_err(|error| anyhow::anyhow!("Missing or invalid parameters: {error}"))?;

        let operation =
            args.operation.as_deref().ok_or_else(|| anyhow::anyhow!("Missing operation"))?;
        if !SUPPORTED_OPERATIONS.contains(&operation) {
            anyhow::bail!("Unsupported LSP operation: {operation}");
        }
        if Self::effective_file_path(&args).is_none() {
            anyhow::bail!("Missing filePath");
        }
        if Self::requires_position(operation) && (args.line.is_none() || args.character.is_none()) {
            anyhow::bail!("Missing line or character");
        }
        Ok(input)
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let result = self.call(args).await?;
        Ok(ToolResult {
            success: result.is_success(),
            output: result.model_text(),
            error: result.error_text(),
        })
    }

    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        if !input.is_object() {
            anyhow::bail!("Missing or invalid parameters: expected object");
        }
        let args: Args = serde_json::from_value(input)
            .map_err(|error| anyhow::anyhow!("Missing or invalid parameters: {error}"))?;
        let validated = self.validate_args(&args).await?;
        match self.execute_internal(&validated).await {
            Ok(result) => Ok(result),
            Err(error) => Ok(self.failure_result(&validated, &error.to_string(), None)),
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
