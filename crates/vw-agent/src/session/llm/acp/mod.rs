//! ACP 会话适配层的聚合模块。
//!
//! 该模块把 VibeWindow 内部的 LLM 流式请求转换为 Agent Client Protocol
//! (ACP) 客户端调用，并集中暴露配置解析、历史重放、会话缓存和更新转换等窄接口。
//! 这里不直接承载传输细节，而是把具体职责拆到相邻子模块中，便于请求流程保持清晰。

#[allow(unused_imports)]
pub(crate) use config::{
    build_acp_command_line, lookup_acp_command, normalize_acp_agent_config, parse_acp_options,
};
#[allow(unused_imports)]
pub(crate) use replay::{build_replay_prompt, parse_replay_strategy};
#[allow(unused_imports)]
pub(crate) use request::do_stream_request_acp;
#[allow(unused_imports)]
pub(crate) use session::{acp_session_name, missing_session_error};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
#[allow(unused_imports)]
pub(crate) use updates::{
    extract_delta_from_acp_message, extract_reasoning_delta_from_acp_message,
    extract_tool_call_from_acp_message,
};

use parking_lot::Mutex;
use tokio::sync::mpsc;
use vw_acp::{
    AcpClient, AcpJsonRpcMessage, AcpSessionOptions, AuthPolicy, NonInteractivePermissionPolicy,
    PermissionMode,
};

use crate::app::agent::session::message;

use super::types::{Error, StreamEvent, ToolCall};

mod approval;
mod config;
mod content;
pub(crate) mod controls;
mod fork_replay;
pub(super) mod lifecycle;
mod model_source;
mod recovery;
mod replay;
pub(super) mod request;
mod session;
mod skill_mounts;
mod updates;

static ACP_CLIENT_CACHE: LazyLock<Mutex<HashMap<String, Arc<CachedAcpClient>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 绑定到一组 ACP 启动配置的客户端缓存项。
///
/// 同一个 ACP 代理进程会复用客户端连接；`prompt_lock` 用来串行化同一客户端上的 prompt，
/// `output_tx` 则在请求运行期间临时接收 ACP JSON-RPC 输出回调。
struct CachedAcpClient {
    permission_confirmation: Arc<parking_lot::Mutex<Option<vw_acp::PermissionConfirmation>>>,
    tenant: vw_api_types::id::TenantScope,
    runtime_identity: Option<String>,
    client: Arc<AcpClient>,
    prompt_lock: Arc<tokio::sync::Mutex<()>>,
    output_tx: Arc<Mutex<Option<mpsc::UnboundedSender<AcpJsonRpcMessage>>>>,
}

/// ACP 客户端缓存命中结果。
///
/// `key` 用于在一次性请求结束后安全移除对应缓存项，`client` 是实际共享客户端。
struct CachedAcpClientEntry {
    request_owned: bool,
    key: String,
    client: Arc<CachedAcpClient>,
}

/// 从运行时选项解析出的 ACP 会话参数。
///
/// 字段保持为 `Option`，让调用方能区分“用户未配置”和“明确配置为空”的场景。
#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct ParsedAcpOptions {
    /// 已在网关按作用域解析的 MCP 配置，不接受请求中的原始凭据。
    pub(super) runtime_identity: Option<String>,
    /// Canonical workspace bound by the desktop turn context.
    pub(super) desktop_workspace: Option<std::path::PathBuf>,
    pub(super) skill_mounts: Vec<vw_acp::SessionSkillMount>,
    pub(super) skill_view: Option<skill_mounts::MountedSkills>,
    pub(super) mcp_servers: Vec<agent_client_protocol::McpServer>,
    /// ACP 权限模式；未设置时由客户端构造逻辑选择默认策略。
    pub(super) permission_mode: Option<PermissionMode>,
    /// 是否允许 ACP 文件系统操作越过会话工作目录边界。
    pub(super) full_access: bool,
    /// 非交互环境下的权限处理策略。
    pub(super) non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    /// ACP 代理认证策略。
    pub(super) auth_policy: Option<AuthPolicy>,
    /// 期望切换到的 ACP 会话模式。
    pub(super) session_mode: Option<String>,
    /// 会话级模型、工具白名单和最大轮次等选项。
    pub(super) session_options: Option<AcpSessionOptions>,
    /// 需要逐项写入 ACP 会话配置的键值对。
    pub(super) session_config_options: Vec<(String, String)>,
}

// MCP payloads may contain credentials; diagnostics expose only their count.
impl std::fmt::Debug for ParsedAcpOptions {
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
pub(super) enum AcpReplayStrategy {
    /// 丢弃旧历史，仅发送当前用户请求和系统约束。
    Discard,
    /// 完整重放本地会话历史。
    Full,
    /// 生成压缩摘要并附带最近若干轮。
    Summary,
    /// 只重放最近若干轮对话。
    Recent,
}

/// 将底层 ACP 错误包装为上层统一的 API 错误。
///
/// 返回值始终是 `Error::Api`，错误文本带 `acp:` 前缀，便于 UI 和日志定位来源。
fn to_api_error(err: impl std::fmt::Display) -> Error {
    Error::Api(message::AssistantError::Unknown { message: format!("acp: {err}") })
}

/// 判断错误文本是否表示 ACP 代理端会话已经发生变化。
///
/// 参数是任意错误消息文本；返回 `true` 时请求层会尝试新建会话并重放上下文。
pub(super) fn is_acp_session_changed_message(message: &str) -> bool {
    message.to_ascii_lowercase().contains("acp session changed:")
}

/// 构造 ACP 选项解析错误。
///
/// 返回值统一归入 API 错误，避免配置错误在调用链上变成未分类失败。
fn acp_option_error(message: impl Into<String>) -> Error {
    Error::Api(message::AssistantError::Unknown {
        message: format!("acp option error: {}", message.into()),
    })
}

/// 从统一错误类型中识别 ACP 会话切换错误。
///
/// 该检查只解析已知 API 错误分支；其他错误类型保持原样上抛，避免误判导致重复请求。
fn is_acp_session_changed_error(err: &Error) -> bool {
    match err {
        Error::Api(message::AssistantError::Unknown { message }) => {
            is_acp_session_changed_message(message)
        }
        Error::Api(message::AssistantError::APIError { message, .. }) => {
            is_acp_session_changed_message(message)
        }
        _ => false,
    }
}
#[cfg(test)]
mod tests;

pub(crate) mod source_selection;
