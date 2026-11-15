//! 共享聊天推理强度与会话代理身份契约。

use serde::{Deserialize, Serialize};

/// 单次调用覆盖的推理强度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningEffortDto {
    /// 显式关闭推理。
    None,
    /// 最低推理强度。
    Minimal,
    /// 低推理强度。
    Low,
    /// 中等推理强度。
    Medium,
    /// 高推理强度。
    High,
    /// 超高推理强度。
    Xhigh,
    /// 最大推理强度。
    Max,
}

/// Read-only UI projection of a server-owned session agent binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionAgentIdentity {
    pub agent: String,
    pub label: Option<String>,
    /// Absent for the native engine; otherwise a stable ACP backend key.
    pub acp_agent: Option<String>,
}

#[cfg(test)]
mod tests;
