//! 宿主无关的 ACP 会话后端边界。

use std::path::PathBuf;

use agent_client_protocol::McpServer;
use async_trait::async_trait;
use std::sync::Arc;

use super::content::PromptContent;
use super::events::BackendEvent;

/// 后端可呈现的权限选项类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendPermissionOptionKind {
    /// 仅允许本次调用。
    AllowOnce,
    /// 仅拒绝本次调用。
    RejectOnce,
}

/// 后端提交给 ACP 客户端的权限选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendPermissionOption {
    /// 仅在本次请求内唯一的选项 ID。
    pub id: String,
    /// 用户可读名称。
    pub name: String,
    /// 选项语义。
    pub kind: BackendPermissionOptionKind,
}

/// 单个后端权限请求及其一次性响应通道。
#[derive(Debug)]
pub struct BackendPermissionRequest {
    /// 原生审批请求 ID。
    pub request_id: String,
    /// 稳定工具调用 ID。
    pub call_id: String,
    /// 用户可读工具标题。
    pub title: String,
    /// 已由原生工具边界脱敏的输入。
    pub raw_input: serde_json::Value,
    /// 本次请求允许返回的选项。
    pub options: Vec<BackendPermissionOption>,
    /// 客户端选择；`None` 表示取消或任何异常退出。
    pub response: tokio::sync::oneshot::Sender<Option<String>>,
}

/// 协议中立的历史消息角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendRole {
    /// 用户消息。
    User,
    /// 助手消息。
    Assistant,
}

/// 可由协议层安全重放的文本历史项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendHistoryEntry {
    /// 消息角色。
    pub role: BackendRole,
    /// 文本内容。
    pub text: String,
}

/// 后端返回的稳定会话身份、作用域和历史。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendSession {
    /// 宿主原生会话 ID。
    pub id: String,
    /// 会话所属的规范化工作目录。
    pub cwd: PathBuf,
    /// 已完成归属校验的历史。
    pub history: Vec<BackendHistoryEntry>,
}

/// 会话后端的分类错误。
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    /// 会话不存在。
    #[error("session not found: {0}")]
    NotFound(String),
    /// 会话不属于请求作用域。
    #[error("session ownership mismatch: {0}")]
    OwnershipMismatch(String),
    /// 请求参数不受后端支持。
    #[error("invalid backend request: {0}")]
    InvalidRequest(String),
    /// 后端内部错误。
    #[error("backend failure: {0}")]
    Internal(String),
}

/// 可重复调用的后端提示取消句柄。
pub trait BackendCancelHandle: Send + Sync + 'static {
    /// 首次实际取消返回 `true`。
    fn cancel(&self) -> bool;
}

/// 已启动的后端提示事件流。
pub struct BackendPrompt {
    /// 单生产者事件通道。
    pub events: tokio::sync::mpsc::UnboundedReceiver<BackendEvent>,
    /// 后端工具审批请求。
    pub permissions: tokio::sync::mpsc::UnboundedReceiver<BackendPermissionRequest>,
    /// 幂等取消句柄。
    pub cancel: Arc<dyn BackendCancelHandle>,
}

impl BackendPrompt {
    /// 构造提示事件流。
    pub fn new(
        events: tokio::sync::mpsc::UnboundedReceiver<BackendEvent>,
        cancel: Arc<dyn BackendCancelHandle>,
    ) -> Self {
        let (permissions, closed_permissions) = tokio::sync::mpsc::unbounded_channel();
        drop(permissions);
        Self { events, permissions: closed_permissions, cancel }
    }

    /// 为提示轮次附加权限请求通道。
    pub fn with_permissions(
        mut self,
        permissions: tokio::sync::mpsc::UnboundedReceiver<BackendPermissionRequest>,
    ) -> Self {
        self.permissions = permissions;
        self
    }
}

/// `vw-acp` Agent 侧依赖的最小宿主接口。
#[async_trait]
pub trait SessionBackend: Send + Sync + 'static {
    /// 在规范化工作目录中创建原生会话。
    async fn create_session(&self, cwd: PathBuf) -> Result<BackendSession, BackendError>;

    /// 在规范化工作目录中创建带会话级 MCP 配置的原生会话。
    async fn create_session_with_mcp(
        &self,
        cwd: PathBuf,
        mcp_servers: Vec<McpServer>,
    ) -> Result<BackendSession, BackendError> {
        if !mcp_servers.is_empty() {
            return Err(BackendError::InvalidRequest(
                "MCP servers are not supported by this backend".to_string(),
            ));
        }
        self.create_session(cwd).await
    }

    /// 在规范化工作目录作用域内加载原生会话。
    async fn load_session(
        &self,
        session_id: &str,
        cwd: PathBuf,
    ) -> Result<BackendSession, BackendError>;

    /// 在规范化目录作用域内加载并重建会话级 MCP 配置。
    async fn load_session_with_mcp(
        &self,
        session_id: &str,
        cwd: PathBuf,
        mcp_servers: Vec<McpServer>,
    ) -> Result<BackendSession, BackendError> {
        if !mcp_servers.is_empty() {
            return Err(BackendError::InvalidRequest(
                "MCP servers are not supported by this backend".to_string(),
            ));
        }
        self.load_session(session_id, cwd).await
    }

    /// 启动一次提示轮次。
    async fn start_prompt(
        &self,
        _session_id: &str,
        _prompt: Vec<PromptContent>,
    ) -> Result<BackendPrompt, BackendError> {
        Err(BackendError::InvalidRequest(
            "prompt execution is not implemented by this backend".to_string(),
        ))
    }
}
