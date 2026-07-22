//! MCP gateway inspection DTOs.

use crate::tools::ToolSpecDto;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP server connection status reported by the gateway inspection endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpConnectionStatusDto {
    Disabled,
    Connected,
    Error,
}

/// Per-server MCP inspection result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpServerInspectDto {
    pub name: String,
    pub transport: String,
    pub enabled: bool,
    pub required: bool,
    pub connection_status: McpConnectionStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub denied_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_template_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<ToolSpecDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_templates: Vec<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompts: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_templates_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompts_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// MCP inspection response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpInspectResponse {
    pub servers: Vec<McpServerInspectDto>,
}
