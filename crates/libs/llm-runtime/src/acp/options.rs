use acp_types::{AcpSessionOptions, AuthPolicy, NonInteractivePermissionPolicy, PermissionMode};
#[derive(Clone, PartialEq, Eq)]
pub struct ParsedAcpOptions<S> {
    /// 已在网关按作用域解析的 MCP 配置，不接受请求中的原始凭据。
    pub runtime_identity: Option<String>,
    /// Canonical workspace bound by the desktop turn context.
    pub desktop_workspace: Option<std::path::PathBuf>,
    pub skill_mounts: Vec<acp_types::SessionSkillMount>,
    pub skill_view: Option<S>,
    pub mcp_servers: Vec<agent_client_protocol::McpServer>,
    /// ACP 权限模式；未设置时由客户端构造逻辑选择默认策略。
    pub permission_mode: Option<PermissionMode>,
    /// 是否允许 ACP 文件系统操作越过会话工作目录边界。
    pub full_access: bool,
    /// 非交互环境下的权限处理策略。
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    /// ACP 代理认证策略。
    pub auth_policy: Option<AuthPolicy>,
    /// 期望切换到的 ACP 会话模式。
    pub session_mode: Option<String>,
    /// 会话级模型、工具白名单和最大轮次等选项。
    pub session_options: Option<AcpSessionOptions>,
    /// 需要逐项写入 ACP 会话配置的键值对。
    pub session_config_options: Vec<(String, String)>,
}

// MCP payloads may contain credentials; diagnostics expose only their count.
impl<S> std::fmt::Debug for ParsedAcpOptions<S> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ParsedAcpOptions")
            .field("mcp_server_count", &self.mcp_servers.len())
            .field("permission_mode", &self.permission_mode)
            .field("full_access", &self.full_access)
            .finish_non_exhaustive()
    }
}

/// 新建 ACP 会话时本地历史的重放策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpReplayStrategy {
    /// 丢弃旧历史，仅发送当前用户请求和系统约束。
    Discard,
    /// 完整重放本地会话历史。
    Full,
    /// 生成压缩摘要并附带最近若干轮。
    Summary,
    /// 只重放最近若干轮对话。
    Recent,
}

impl<S> Default for ParsedAcpOptions<S> { fn default() -> Self { Self { runtime_identity: Default::default(), desktop_workspace: Default::default(), skill_mounts: Default::default(), skill_view: Default::default(), mcp_servers: Default::default(), permission_mode: Default::default(), full_access: Default::default(), non_interactive_permissions: Default::default(), auth_policy: Default::default(), session_mode: Default::default(), session_options: Default::default(), session_config_options: Default::default() } } }
