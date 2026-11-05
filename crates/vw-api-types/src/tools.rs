//! Claude Tools V2 共享 DTO。
//!
//! 本模块为工具规格、工具调用与工具结果提供稳定的跨 crate 协议类型，
//! 供 agent、gateway、ACP、desktop 等边界层复用。

use crate::id::ToolId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

fn default_strict_tool_schema() -> bool {
    true
}

/// 协议身份独立于稳定授权 ID 及展示名。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ToolIdentityWire")]
pub struct ToolIdentity {
    pub namespace: Option<String>,
    pub name: String,
}

#[derive(Deserialize)]
struct ToolIdentityWire {
    namespace: Option<String>,
    name: String,
}

impl TryFrom<ToolIdentityWire> for ToolIdentity {
    type Error = String;

    fn try_from(value: ToolIdentityWire) -> Result<Self, Self::Error> {
        Self::new(value.namespace, value.name)
    }
}

impl ToolIdentity {
    pub fn new(namespace: Option<String>, name: impl Into<String>) -> Result<Self, String> {
        let identity = Self { namespace, name: name.into() };
        identity.validate()?;
        Ok(identity)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.namespace.as_ref().is_some_and(|name| name.trim().is_empty())
        {
            return Err("tool identity must not be empty".into());
        }
        Ok(())
    }
}

/// 来源是审计元数据，不是客户端可授予的安全能力。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSource {
    #[default]
    Builtin,
    Mcp,
    Client,
}

/// Direct/Deferred 同时允许 Code Mode；ModelOnly 排除嵌套代码调用。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExposure {
    #[default]
    Direct,
    Deferred,
    DirectModelOnly,
    DeferredModelOnly,
    CodeModeOnly,
    Hidden,
}

impl ToolExposure {
    pub fn is_direct(self) -> bool {
        matches!(self, Self::Direct | Self::DirectModelOnly)
    }

    pub fn is_deferred(self) -> bool {
        matches!(self, Self::Deferred | Self::DeferredModelOnly)
    }

    pub fn is_available_in_code_mode(self) -> bool {
        matches!(self, Self::Direct | Self::Deferred | Self::CodeModeOnly)
    }
}

/// 客户端注册按函数或命名空间分组；宿主在接纳前必须校验并按稳定 ID 授权。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DynamicToolRegistration {
    Function(ToolSpecDto),
    Namespace { name: String, description: String, tools: Vec<ToolSpecDto> },
}

impl DynamicToolRegistration {
    /// 校验外部注册，拒绝来源伪装、保留名、重复身份和重复 ID。
    pub fn validate(&self) -> Result<(), String> {
        let (namespace, tools) = match self {
            Self::Function(tool) => (None, std::slice::from_ref(tool)),
            Self::Namespace { name, tools, .. } => {
                if name.trim().is_empty() || reserved_tool_name(name) || tools.is_empty() {
                    return Err("invalid dynamic tool namespace".into());
                }
                (Some(name), tools.as_slice())
            }
        };
        let mut ids = std::collections::BTreeSet::new();
        let mut identities = std::collections::BTreeSet::new();
        for tool in tools {
            tool.identity.validate()?;
            if tool.source != ToolSource::Client
                || tool.id.0.as_str().trim().is_empty()
                || reserved_tool_name(tool.id.0.as_str())
                || reserved_tool_name(&tool.identity.name)
                || tool.identity.namespace.as_deref().is_some_and(reserved_tool_name)
                || !tool.aliases.is_empty()
                || !tool.input_schema.is_object()
            {
                return Err("invalid client tool registration".into());
            }
            if namespace
                .is_some_and(|namespace| tool.identity.namespace.as_ref() != Some(namespace))
            {
                return Err("tool identity does not match its namespace".into());
            }
            if !ids.insert(tool.id.0.as_str())
                || !identities.insert((&tool.identity.namespace, &tool.identity.name))
            {
                return Err("duplicate dynamic tool ID or identity".into());
            }
        }
        Ok(())
    }
}

fn reserved_tool_name(name: &str) -> bool {
    matches!(name, "tool_search" | "exec" | "wait")
}

/// 客户端回调请求必须携带宿主解析的稳定 ID。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicToolCallRequest {
    pub call_id: String,
    pub turn_id: String,
    pub tool_id: ToolId,
    pub identity: ToolIdentity,
    pub arguments: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DynamicToolResponse {
    pub success: bool,
    pub content: Vec<DynamicToolContent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DynamicToolContent {
    Text { text: String },
    Image { image_url: String },
    Audio { audio_url: String },
}

/// V2 工具规格 DTO。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpecDto {
    pub identity: ToolIdentity,
    pub source: ToolSource,
    pub exposure: ToolExposure,
    /// 稳定工具 ID。
    pub id: ToolId,
    /// 工具展示名。
    pub display_name: String,
    /// 工具描述。
    pub description: String,
    /// 输入 schema。
    pub input_schema: Value,
    /// 可接受的别名。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// 是否只读。
    #[serde(default)]
    pub read_only: bool,
    /// 是否包含破坏性写操作。
    #[serde(default)]
    pub destructive: bool,
    /// 是否允许并发执行。
    #[serde(default)]
    pub concurrency_safe: bool,
    /// 是否需要用户交互。
    #[serde(default)]
    pub requires_user_interaction: bool,
    /// 是否要求严格 schema。
    #[serde(default = "default_strict_tool_schema")]
    pub strict: bool,
}

/// 工具规格列表响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListToolSpecsResponse {
    /// 工具规格列表。
    pub items: Vec<ToolSpecDto>,
}

/// 工具使用 DTO。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolUseDto {
    /// 工具使用 ID，通常对应 provider 的 tool_call_id。
    pub id: String,
    /// 被调用的工具 ID。
    pub tool_id: ToolId,
    /// 工具参数。
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub arguments: Value,
}

/// 结构化补丁 hunk DTO。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredPatchHunkDto {
    /// hunk 头部，例如 @@ -1,3 +1,4 @@。
    pub header: String,
    /// 关联文件路径。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// 旧文件起始行。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_start: Option<u32>,
    /// 旧文件影响行数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_lines: Option<u32>,
    /// 新文件起始行。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_start: Option<u32>,
    /// 新文件影响行数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_lines: Option<u32>,
    /// hunk 内部的逐行内容。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<String>,
}

/// 工具渲染提示 DTO。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RenderHintDto {
    /// 展示标题。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// 渲染种类。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// 摘要。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// 结构化扩展元数据。
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

/// 审批请求 DTO。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionRequestDto {
    /// 需要审批的原因。
    pub reason: String,
    /// 可选警告信息。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    /// 审批前最终归一化后的输入。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_input: Option<Value>,
}

/// 工具结果内容 DTO。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContentDto {
    /// 纯文本结果。
    Text { text: String },
    /// 任意 JSON 结果。
    Json { value: Value },
    /// 结构化补丁结果。
    StructuredPatch { hunks: Vec<StructuredPatchHunkDto> },
}

/// V2 工具结果 DTO。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResultDto {
    /// 对应的 tool use id。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    /// 对应的工具 ID。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_id: Option<ToolId>,
    /// 调用是否成功。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    /// 面向消费层的结果内容块。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ToolResultContentDto>,
    /// 工具内部结构化数据。
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub data: Value,
    /// 面向模型的结果。
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub model_result: Value,
    /// 渲染提示。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_hint: Option<RenderHintDto>,
    /// 可选审批请求。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_request: Option<PermissionRequestDto>,
    /// 写回上下文的更新。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_updates: Vec<Value>,
    /// 需要追加的额外消息。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_messages: Vec<Value>,
    /// 遥测扩展字段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<Value>,
}

/// Native session host binding. Send the token in x-vibewindow-tool-host, never in URLs.
/// Deliberately does not implement Debug: the token is a live capability.
#[derive(Clone, Serialize, Deserialize)]
pub struct DynamicToolHostBinding {
    pub host_id: String,
    pub binding_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaceDynamicToolsRequest {
    pub tools: Vec<DynamicToolRegistration>,
    #[serde(default)]
    pub persist: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceDynamicToolsResponse {
    pub version: u64,
}
