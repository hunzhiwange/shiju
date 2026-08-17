//! 工具核心 trait 定义
//!
//! 本模块承载 Claude Tools V2 的基础契约，统一定义：
//! - 工具对模型暴露的规格描述 [`ToolSpec`]
//! - 工具调用后的结构化结果 [`ToolCallResult`]
//! - 工具实现需要满足的运行时接口 [`Tool`]
//!
//! 目前仓库中的大部分工具仍然实现旧的 `name / description / parameters_schema /
//! execute` 四元组。本模块在 V2 trait 上保留这些窄接口作为实现适配层，确保本轮
//! 先完成运行时契约切换，而不把 60 多个具体工具实现一起卷入同一批补丁。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use vw_api_types::tools::{
    PermissionRequestDto, RenderHintDto, ToolResultContentDto, ToolResultDto, ToolSpecDto,
};

/// 旧工具执行结果。
///
/// 该结构仍保留给现有工具实现直接返回，V2 运行时会在调用边界把它提升为
/// [`ToolCallResult`]。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolResult {
    /// 执行是否成功。
    pub success: bool,
    /// 文本输出。
    pub output: String,
    /// 错误信息。
    pub error: Option<String>,
}

/// Claude Tools V2 工具规格。
///
/// 新字段以 `id / input_schema` 为主；`name / parameters` 仅作为当前消费层的镜像字段，
/// 用于本阶段平滑切换 provider、prompt 与测试代码，后续协议切换阶段会统一收口。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// 稳定工具 ID。
    pub id: String,
    /// 用于 UI 或审计展示的名称。
    pub display_name: String,
    /// 工具描述。
    pub description: String,
    /// V2 输入 schema。
    pub input_schema: Value,
    /// 兼容旧消费面的镜像名称。
    #[serde(default)]
    pub name: String,
    /// 兼容旧消费面的镜像参数 schema。
    #[serde(default)]
    pub parameters: Value,
    /// 可接受的别名集合。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// 是否只读。
    #[serde(default)]
    pub read_only: bool,
    /// 是否包含破坏性写操作。
    #[serde(default)]
    pub destructive: bool,
    /// 是否允许和同批其它调用并发执行。
    #[serde(default)]
    pub concurrency_safe: bool,
    /// 是否需要用户交互。
    #[serde(default)]
    pub requires_user_interaction: bool,
    /// 是否强制使用严格 schema。
    #[serde(default = "default_strict_tool_schema")]
    pub strict: bool,
}

fn default_strict_tool_schema() -> bool {
    true
}

impl Default for ToolSpec {
    fn default() -> Self {
        Self {
            id: String::new(),
            display_name: String::new(),
            description: String::new(),
            input_schema: Value::Null,
            name: String::new(),
            parameters: Value::Null,
            aliases: Vec::new(),
            read_only: false,
            destructive: false,
            concurrency_safe: false,
            requires_user_interaction: false,
            strict: true,
        }
    }
}

impl ToolSpec {
    /// 使用最小必需信息构造 V2 工具规格。
    pub fn new(id: impl Into<String>, description: impl Into<String>, input_schema: Value) -> Self {
        let id = id.into();
        let input_schema_clone = input_schema.clone();
        Self {
            name: id.clone(),
            parameters: input_schema_clone,
            display_name: id.clone(),
            description: description.into(),
            input_schema,
            id,
            ..Self::default()
        }
    }

    /// 转换为跨模块共享的工具规格 DTO。
    pub fn to_dto(&self) -> ToolSpecDto {
        ToolSpecDto {
            id: self.id.clone().into(),
            display_name: self.display_name.clone(),
            description: self.description.clone(),
            input_schema: self.input_schema.clone(),
            aliases: self.aliases.clone(),
            read_only: self.read_only,
            destructive: self.destructive,
            concurrency_safe: self.concurrency_safe,
            requires_user_interaction: self.requires_user_interaction,
            strict: self.strict,
        }
    }

    /// 设置展示名称。
    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    /// 设置别名列表。
    pub fn with_aliases<I, S>(mut self, aliases: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.aliases = aliases.into_iter().map(Into::into).collect();
        self
    }

    /// 设置只读标记。
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// 设置破坏性标记。
    pub fn with_destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    /// 设置并发安全标记。
    pub fn with_concurrency_safe(mut self, concurrency_safe: bool) -> Self {
        self.concurrency_safe = concurrency_safe;
        self
    }

    /// 设置用户交互标记。
    pub fn with_requires_user_interaction(mut self, requires_user_interaction: bool) -> Self {
        self.requires_user_interaction = requires_user_interaction;
        self
    }

    /// 设置 schema 严格模式。
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }
}

/// 工具调用后的上下文回写描述。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolContextUpdate {
    /// 更新键。
    pub key: String,
    /// 更新值。
    #[serde(default)]
    pub value: Value,
}

/// 工具调用额外注入的消息。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolExtraMessage {
    /// 角色。
    pub role: String,
    /// 内容。
    #[serde(default)]
    pub content: Value,
}

impl ToolExtraMessage {
    /// 创建仅供模型上下文消费的元用户文本消息。
    ///
    /// 技能正文不能混入普通工具结果，否则模型会把加载确认和执行指令混为一谈。
    pub fn meta_user_text(content: impl Into<String>) -> Self {
        Self { role: "user".to_string(), content: Value::String(content.into()) }
    }
}

/// 工具渲染提示。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolRenderHint {
    /// 展示标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 渲染种类。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// 简要摘要。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// 结构化渲染元数据。
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

impl ToolRenderHint {
    /// 创建仅包含标题的渲染提示。
    pub fn titled(title: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            metadata: Value::Object(Default::default()),
            ..Self::default()
        }
    }

    /// 转换为共享渲染提示 DTO。
    pub fn to_dto(&self) -> RenderHintDto {
        RenderHintDto {
            title: self.title.clone(),
            kind: self.kind.clone(),
            summary: self.summary.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

/// 工具调用遥测信息。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCallTelemetry {
    /// 调用是否成功。
    #[serde(default)]
    pub success: bool,
    /// 运行过程中产生的告警。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// 扩展属性。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, Value>,
}

/// Claude Tools V2 结构化工具结果。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCallResult {
    /// 面向系统内部的结构化数据。
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub data: Value,
    /// 面向模型回填的结果。
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub model_result: Value,
    /// 面向 ACP / gateway / UI 的结构化内容块。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_blocks: Vec<ToolResultContentDto>,
    /// 面向 UI / ACP / desktop 的渲染提示。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_hint: Option<ToolRenderHint>,
    /// 可选的权限请求上下文。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_request: Option<PermissionRequestDto>,
    /// 需要写回调用上下文的更新。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_updates: Vec<ToolContextUpdate>,
    /// 需要追加到消息历史的额外消息。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_messages: Vec<ToolExtraMessage>,
    /// 调用遥测。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<ToolCallTelemetry>,
}

impl ToolCallResult {
    /// 将旧的文本结果提升为 V2 结构化结果。
    pub fn from_legacy_result(result: ToolResult) -> Self {
        let success = result.success;
        let output = result.output;
        let error = result.error;
        let model_text =
            if success { output.clone() } else { error.clone().unwrap_or_else(|| output.clone()) };

        Self {
            data: json!({
                "success": success,
                "output": output,
                "error": error,
            }),
            model_result: Value::String(model_text),
            telemetry: Some(ToolCallTelemetry { success, ..ToolCallTelemetry::default() }),
            ..Self::default()
        }
    }

    /// 转换为共享工具结果 DTO。
    pub fn to_dto(&self) -> ToolResultDto {
        self.to_dto_with_meta(None, None)
    }

    /// 转换为共享工具结果 DTO，并补充工具与调用标识。
    pub fn to_dto_with_meta(
        &self,
        tool_id: Option<&str>,
        tool_use_id: Option<&str>,
    ) -> ToolResultDto {
        let model_result = self.default_model_result();
        let mut content = self.content_blocks.clone();

        if content.is_empty() {
            match &model_result {
                Value::String(text) if !text.trim().is_empty() => {
                    content.push(ToolResultContentDto::Text { text: text.clone() });
                }
                Value::Null => {}
                other => content.push(ToolResultContentDto::Json { value: other.clone() }),
            }
        }

        if content.is_empty() && !self.data.is_null() {
            content.push(ToolResultContentDto::Json { value: self.data.clone() });
        }

        ToolResultDto {
            tool_use_id: tool_use_id.map(ToOwned::to_owned),
            tool_id: tool_id.map(|id| id.to_string().into()),
            success: Some(self.is_success()),
            content,
            data: self.data.clone(),
            model_result,
            render_hint: self.render_hint.as_ref().map(ToolRenderHint::to_dto),
            permission_request: self.permission_request.clone(),
            context_updates: self
                .context_updates
                .iter()
                .filter_map(|update| serde_json::to_value(update).ok())
                .collect(),
            extra_messages: self
                .extra_messages
                .iter()
                .filter_map(|message| serde_json::to_value(message).ok())
                .collect(),
            telemetry: self
                .telemetry
                .as_ref()
                .and_then(|telemetry| serde_json::to_value(telemetry).ok()),
        }
    }

    /// 判断结果是否成功。
    pub fn is_success(&self) -> bool {
        self.telemetry
            .as_ref()
            .map(|telemetry| telemetry.success)
            .or_else(|| self.data.get("success").and_then(Value::as_bool))
            .unwrap_or_else(|| {
                !self.data.get("error").is_some_and(|error| {
                    if error.is_null() {
                        return false;
                    }
                    if let Some(text) = error.as_str() {
                        return !text.trim().is_empty();
                    }
                    true
                })
            })
    }

    /// 生成默认的模型结果。
    pub fn default_model_result(&self) -> Value {
        if !self.model_result.is_null() {
            return self.model_result.clone();
        }

        if let Some(output) = self.data.get("output") {
            return output.clone();
        }

        if let Some(error) = self.data.get("error") {
            return error.clone();
        }

        if !self.data.is_null() {
            return self.data.clone();
        }

        Value::String(String::new())
    }

    /// 将模型结果压平成文本。
    pub fn model_text(&self) -> String {
        match self.default_model_result() {
            Value::String(text) => text,
            Value::Null => String::new(),
            other => serde_json::to_string_pretty(&other).unwrap_or_else(|_| other.to_string()),
        }
    }

    /// 提取错误文本。
    pub fn error_text(&self) -> Option<String> {
        if let Some(error) = self.data.get("error").and_then(Value::as_str) {
            return Some(error.to_string());
        }

        if self.is_success() {
            return None;
        }

        let text = self.model_text();
        (!text.trim().is_empty()).then_some(text)
    }

    /// 读取渲染标题。
    pub fn render_title(&self, fallback: &str) -> String {
        self.render_hint
            .as_ref()
            .and_then(|hint| hint.title.clone())
            .unwrap_or_else(|| fallback.to_string())
    }

    /// 读取渲染元数据。
    pub fn render_metadata(&self) -> Value {
        self.render_hint
            .as_ref()
            .map(|hint| {
                if hint.metadata.is_null() {
                    Value::Object(Default::default())
                } else {
                    hint.metadata.clone()
                }
            })
            .unwrap_or_else(|| Value::Object(Default::default()))
    }
}

/// 工具 trait 边界（非 WASM 目标）
///
/// 在非 WASM 目标平台上，要求工具实现 `Send + Sync` 以支持跨线程安全共享。
/// 这是通过条件编译实现的平台相关 trait 边界定义。
#[cfg(not(target_arch = "wasm32"))]
pub trait ToolBounds: Send + Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + Sync> ToolBounds for T {}

/// 工具 trait 边界（WASM 目标）
///
/// 在 WASM 目标平台上，由于单线程模型限制，不要求 `Send + Sync`。
/// 这允许在 WebAssembly 环境中使用工具实现。
#[cfg(target_arch = "wasm32")]
pub trait ToolBounds {}
#[cfg(target_arch = "wasm32")]
impl<T> ToolBounds for T {}

/// 工具核心 trait。
///
/// V2 运行时对外使用 `spec / validate_input / check_permissions / call /
/// map_result_for_model / render_hint` 这一组方法；当前具体工具实现仍可继续实现
/// `name / description / parameters_schema / execute`，由默认适配层接入新契约。
///
/// # 示例
///
/// ```ignore
/// use vibewindow::tools::{Tool, ToolResult};
/// use async_trait::async_trait;
///
/// struct EchoTool;
///
/// #[async_trait]
/// impl Tool for EchoTool {
///     fn name(&self) -> &str { "echo" }
///     fn description(&self) -> &str { "返回输入文本" }
///     fn parameters_schema(&self) -> serde_json::Value {
///         serde_json::json!({
///             "type": "object",
///             "properties": {
///                 "text": {"type": "string"}
///             }
///         })
///     }
///     async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
///         let text = args["text"].as_str().unwrap_or("");
///         Ok(ToolResult { success: true, output: text.to_string(), error: None })
///     }
/// }
/// ```
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait Tool: ToolBounds {
    /// 返回工具名称。
    fn name(&self) -> &str;

    /// 返回工具的人类可读描述。
    fn description(&self) -> &str;

    /// 返回工具参数的 JSON Schema。
    fn parameters_schema(&self) -> Value;

    /// 旧实现层的执行入口。
    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult>;

    /// 对输入做轻量校验与归一化。
    fn validate_input(&self, input: Value) -> anyhow::Result<Value> {
        Ok(input)
    }

    /// 工具自身权限检查。
    async fn check_permissions(&self, _input: &Value) -> anyhow::Result<()> {
        Ok(())
    }

    /// V2 运行时调用入口。
    async fn call(&self, input: Value) -> anyhow::Result<ToolCallResult> {
        let legacy = self.execute(input).await?;
        let mut result = ToolCallResult::from_legacy_result(legacy);
        result.model_result = self.map_result_for_model(&result);
        if result.render_hint.is_none() {
            result.render_hint = self.render_hint(&result);
        }
        Ok(result)
    }

    /// 将结构化结果映射为模型可继续消费的结果片段。
    fn map_result_for_model(&self, result: &ToolCallResult) -> Value {
        result.default_model_result()
    }

    /// 生成渲染提示。
    fn render_hint(&self, result: &ToolCallResult) -> Option<ToolRenderHint> {
        result
            .render_hint
            .clone()
            .or_else(|| Some(ToolRenderHint::titled(self.spec().display_name.clone())))
    }

    /// 是否允许并发执行。
    fn is_concurrency_safe(&self) -> bool {
        default_tool_concurrency_safe(self.name())
    }

    /// 是否只读。
    fn is_read_only(&self) -> bool {
        default_tool_read_only(self.name())
    }

    /// 生成审计输入。
    fn to_audit_input(&self, input: &Value) -> Value {
        input.clone()
    }

    /// 生成完整的 V2 工具规格。
    fn spec(&self) -> ToolSpec {
        let tool_name = self.name();
        ToolSpec::new(tool_name, self.description(), self.parameters_schema())
            .with_display_name(tool_name)
            .with_aliases(default_tool_aliases(tool_name))
            .with_read_only(self.is_read_only())
            .with_destructive(default_tool_destructive(tool_name))
            .with_concurrency_safe(self.is_concurrency_safe())
            .with_requires_user_interaction(default_tool_requires_user_interaction(tool_name))
            .with_strict(true)
    }
}

fn default_tool_aliases(tool_id: &str) -> Vec<String> {
    match tool_id {
        "shell" => vec!["bash".to_string()],
        "edit" => vec!["file_edit".to_string()],
        "file_read" => vec!["read".to_string()],
        "file_write" => vec!["write".to_string()],
        "web_fetch" => vec!["webfetch".to_string()],
        "web_search_tool" => vec!["websearch".to_string()],
        _ => Vec::new(),
    }
}

fn default_tool_read_only(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "file_read"
            | "ls"
            | "glob"
            | "glob_search"
            | "content_search"
            | "grep"
            | "codesearch"
            | "lsp"
            | "memory_recall"
            | "cron_list"
            | "cron_runs"
            | "pdf_read"
            | "image_info"
            | "image_asset_read"
            | "web_fetch"
            | "web_search_tool"
    )
}

fn default_tool_destructive(tool_id: &str) -> bool {
    matches!(
        tool_id,
        "edit"
            | "file_write"
            | "apply_patch"
            | "shell"
            | "process"
            | "git_operations"
            | "cron_add"
            | "cron_remove"
            | "cron_update"
            | "cron_run"
            | "memory_store"
            | "memory_forget"
            | "schedule"
            | "todowrite"
            | "browser"
            | "browser_open"
    )
}

fn default_tool_concurrency_safe(tool_id: &str) -> bool {
    default_tool_read_only(tool_id)
        || matches!(tool_id, "memory_recall")
}

fn default_tool_requires_user_interaction(tool_id: &str) -> bool {
    matches!(tool_id, "question")
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
