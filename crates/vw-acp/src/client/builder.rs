//! `AcpClient` 构造和链式配置。

use super::*;

impl AcpClient {
    /// 创建一个新的 ACP 客户端。
    ///
    /// `agent_name` 用于日志和生命周期标识，`config` 描述实际启动命令。
    /// 返回的客户端尚未启动代理进程；首次会话或提示词请求会按需启动。
    pub fn new(agent_name: impl Into<String>, config: AcpAgentConfig) -> Self {
        Self {
            agent_name: agent_name.into(),
            config,
            client_name: "vibewindow-acp-client".to_string(),
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            mcp_servers: Vec::new(),
            permission_mode: PermissionMode::ApproveAll,
            full_access: false,
            non_interactive_permissions: None,
            auth_credentials: HashMap::new(),
            auth_policy: AuthPolicy::Skip,
            session_options: None,
            verbose: false,
            on_acp_message: None,
            on_acp_output_message: None,
            on_session_update: None,
            on_client_operation: None,
            permission_stats: Arc::new(Mutex::new(PermissionStats::default())),
            active_prompt: Arc::new(Mutex::new(None)),
            cancelling_session_ids: Arc::new(Mutex::new(HashSet::new())),
            actor_state: Arc::new(Mutex::new(AcpClientActorState::default())),
            actor_idle_timeout: DEFAULT_ACTOR_IDLE_TIMEOUT,
        }
    }

    /// 设置初始化握手时上报给 ACP 代理的客户端信息。
    ///
    /// 返回更新后的客户端，便于链式配置。该方法不执行 I/O，也不会校验
    /// 代理是否接受这些元数据。
    pub fn with_client_info(
        mut self,
        client_name: impl Into<String>,
        client_version: impl Into<String>,
    ) -> Self {
        self.client_name = client_name.into();
        self.client_version = client_version.into();
        self
    }

    /// 设置创建、加载或恢复会话时传递给代理的 MCP 服务器列表。
    pub fn with_mcp_servers(mut self, mcp_servers: Vec<acp::McpServer>) -> Self {
        self.mcp_servers = mcp_servers;
        self
    }

    /// 设置文件系统和终端能力的权限模式。
    pub fn with_permission_mode(mut self, permission_mode: PermissionMode) -> Self {
        self.permission_mode = permission_mode;
        self
    }

    /// 设置是否允许文件系统请求访问会话 cwd 子树之外的绝对路径。
    pub fn with_full_access(mut self, full_access: bool) -> Self {
        self.full_access = full_access;
        self
    }

    /// 设置非交互模式下的权限策略。
    ///
    /// 当调用方无法弹出权限确认界面时，该策略决定 ACP 权限请求如何自动处理。
    pub fn with_non_interactive_permissions(
        mut self,
        non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    ) -> Self {
        self.non_interactive_permissions = non_interactive_permissions;
        self
    }

    /// 设置创建新会话时附带的会话选项。
    ///
    /// 当前这些选项会被转换到 ACP `meta` 字段中，主要用于兼容支持该扩展的代理。
    pub fn with_session_options(mut self, session_options: Option<AcpSessionOptions>) -> Self {
        self.session_options = session_options;
        self
    }

    /// 设置用于 ACP 认证的凭据映射。
    ///
    /// 凭据只用于环境变量注入和认证方法选择；日志路径不会输出原始凭据值。
    pub fn with_auth_credentials(mut self, auth_credentials: HashMap<String, String>) -> Self {
        self.auth_credentials = auth_credentials;
        self
    }

    /// 设置代理声明认证方法但找不到凭据时的处理策略。
    pub fn with_auth_policy(mut self, auth_policy: AuthPolicy) -> Self {
        self.auth_policy = auth_policy;
        self
    }

    /// 设置是否启用更详细的诊断日志。
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// 设置原始 ACP JSON-RPC 消息回调。
    pub fn with_acp_message_callback(mut self, callback: Option<AcpMessageCallback>) -> Self {
        self.on_acp_message = callback;
        self
    }

    /// 设置面向输出流的 ACP 消息回调。
    pub fn with_acp_output_message_callback(
        mut self,
        callback: Option<AcpMessageCallback>,
    ) -> Self {
        self.on_acp_output_message = callback;
        self
    }

    /// 设置 ACP 会话更新回调。
    pub fn with_session_update_callback(mut self, callback: Option<SessionUpdateCallback>) -> Self {
        self.on_session_update = callback;
        self
    }

    /// 设置客户端侧文件系统和终端操作回调。
    pub fn with_client_operation_callback(
        mut self,
        callback: Option<ClientOperationCallback>,
    ) -> Self {
        self.on_client_operation = callback;
        self
    }

    #[cfg(test)]
    pub(super) fn with_actor_idle_timeout(mut self, actor_idle_timeout: Duration) -> Self {
        self.actor_idle_timeout = actor_idle_timeout;
        self
    }

    /// 返回最近一次提示词或权限流程累计的权限统计。
    pub fn permission_stats(&self) -> PermissionStats {
        self.permission_stats.lock().clone()
    }
}
