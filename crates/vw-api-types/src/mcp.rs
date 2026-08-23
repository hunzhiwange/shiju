//! MCP gateway inspection DTOs.
//!
//! 定义网关检查 MCP 服务连接状态、能力清单及分项错误时使用的数据契约。

use crate::tools::ToolSpecDto;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP server connection status reported by the gateway inspection endpoint.
/// 网关检查接口报告的 MCP 服务连接状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpConnectionStatusDto {
    /// 服务已配置但被禁用，检查过程不会尝试连接。
    Disabled,
    /// 服务连接成功，可以继续检查其能力。
    Connected,
    /// 连接或初始化失败，具体原因见检查结果。
    Error,
}

/// Per-server MCP inspection result.
/// 单个 MCP 服务的配置、能力计数、资源清单及分项错误。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpServerInspectDto {
    /// MCP 服务配置名称。
    pub name: String,
    /// 服务使用的传输类型。
    pub transport: String,
    /// 服务是否启用。
    pub enabled: bool,
    /// 服务不可用时是否应视为运行时错误。
    pub required: bool,
    /// 本次检查观察到的连接状态。
    pub connection_status: McpConnectionStatusDto,
    /// 为该服务配置的调用超时毫秒数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    /// 允许暴露的工具名称；为空表示未设置允许列表。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    /// 明确禁止暴露的工具名称。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub denied_tools: Vec<String>,
    /// 服务报告的工具数量；未执行该项检查时缺失。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_count: Option<usize>,
    /// 服务报告的资源数量；未执行该项检查时缺失。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_count: Option<usize>,
    /// 服务报告的资源模板数量；未执行该项检查时缺失。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_template_count: Option<usize>,
    /// 服务报告的提示模板数量；未执行该项检查时缺失。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_count: Option<usize>,
    /// 检查成功后返回的工具规格。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolSpecDto>,
    /// MCP 原始资源描述，保留服务端扩展字段。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<Value>,
    /// MCP 原始资源模板描述，保留服务端扩展字段。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_templates: Vec<Value>,
    /// MCP 原始提示模板描述，保留服务端扩展字段。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompts: Vec<Value>,
    /// 工具检查失败时的独立诊断，不影响其它能力结果。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_error: Option<String>,
    /// 资源检查失败时的独立诊断。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources_error: Option<String>,
    /// 资源模板检查失败时的独立诊断。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_templates_error: Option<String>,
    /// 提示模板检查失败时的独立诊断。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts_error: Option<String>,
    /// 连接或服务级失败诊断。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// MCP inspection response.
/// MCP 检查接口响应，按服务返回彼此独立的检查结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpInspectResponse {
    /// 按配置顺序返回的 MCP 服务检查结果。
    pub servers: Vec<McpServerInspectDto>,
}
