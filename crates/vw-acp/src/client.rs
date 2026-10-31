//! ACP 客户端门面。
//!
//! 该模块提供面向上层调用方的 `AcpClient`，负责配置 ACP 代理进程、
//! 管理后台 actor 线程、转发会话控制请求，并暴露提示词执行、取消、
//! 权限统计和生命周期查询等能力。具体的 actor 循环、事件回调和辅助函数
//! 拆分到 `client/*` 子模块中。

use std::collections::{HashMap, HashSet};
use std::error::Error as StdError;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::thread;

use agent_client_protocol::{self as acp, Agent as _};
use parking_lot::Mutex;
use serde_json::{Map, Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};
use tokio::process::Child;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::{Duration, timeout};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use crate::acp_jsonrpc::is_acp_json_rpc_message;
use crate::error::AcpError;
use crate::filesystem::{FileSystemHandlers, FileSystemHandlersOptions};
use crate::permissions::{
    PermissionDecision, classify_permission_decision, resolve_permission_request,
};
use crate::session_persistence::iso_now;
use crate::session_runtime::AgentLifecycleSnapshot;
use crate::spawn_command_options::build_spawn_command;
use crate::terminal::{TerminalManager, TerminalManagerOptions};
use crate::types::{
    AcpAgentConfig, AcpJsonRpcMessage, AcpMessageCallback, AcpMessageDirection, AcpSessionOptions,
    AuthPolicy, ClientOperationCallback, NonInteractivePermissionPolicy, PermissionMode,
    PermissionStats, PromptEvent, PromptRequest, PromptResult, PromptUsage, SessionInfo,
    SessionStrategy, SessionUpdateCallback,
};

#[path = "client/active_prompt.rs"]
mod active_prompt;
#[path = "client/actor.rs"]
mod actor;
#[path = "client/actor_handle.rs"]
mod actor_handle;
#[path = "client/auth.rs"]
mod auth;
#[path = "client/auth_env.rs"]
mod auth_env;
#[path = "client/builder.rs"]
mod builder;
#[path = "client/client_error.rs"]
mod client_error;
#[path = "client/commands.rs"]
mod commands;
#[path = "client/error_context.rs"]
mod error_context;
#[path = "client/event_client.rs"]
mod event_client;
#[path = "client/helpers.rs"]
mod helpers;
#[path = "client/lifecycle.rs"]
mod lifecycle;
#[path = "client/message_io.rs"]
mod message_io;
#[path = "client/process.rs"]
mod process;
#[path = "client/process_signals.rs"]
mod process_signals;
#[path = "client/prompt_controls.rs"]
mod prompt_controls;
#[path = "client/prompt_loop.rs"]
mod prompt_loop;
#[path = "client/prompt_mapping.rs"]
mod prompt_mapping;
#[path = "client/protocol.rs"]
mod protocol;
#[path = "client/runtime.rs"]
mod runtime;
#[path = "client/session_control.rs"]
mod session_control;
#[path = "client/session_meta.rs"]
mod session_meta;
#[path = "client/state.rs"]
mod state;
#[path = "client/timeout_messages.rs"]
mod timeout_messages;

#[cfg(test)]
#[path = "client/active_prompt_tests.rs"]
mod active_prompt_tests;
#[cfg(test)]
#[path = "client/actor_handle_tests.rs"]
mod actor_handle_tests;
#[cfg(test)]
#[path = "client/auth_env_tests.rs"]
mod auth_env_tests;
#[cfg(test)]
#[path = "client/auth_tests.rs"]
mod auth_tests;
#[cfg(test)]
#[path = "client/builder_tests.rs"]
mod builder_tests;
#[cfg(test)]
#[path = "client/client_error_tests.rs"]
mod client_error_tests;
#[cfg(test)]
#[path = "client/commands_tests.rs"]
mod commands_tests;
#[cfg(test)]
#[path = "client/error_context_tests.rs"]
mod error_context_tests;
#[cfg(test)]
#[path = "client/event_client_tests.rs"]
mod event_client_tests;
#[cfg(test)]
#[path = "client/helpers_tests.rs"]
mod helpers_tests;
#[cfg(test)]
#[path = "client/lifecycle_tests.rs"]
mod lifecycle_tests;
#[cfg(test)]
#[path = "client/message_io_tests.rs"]
mod message_io_tests;
#[cfg(test)]
#[path = "client/process_signals_tests.rs"]
mod process_signals_tests;
#[cfg(test)]
#[path = "client/process_tests.rs"]
mod process_tests;
#[cfg(test)]
#[path = "client/prompt_loop_tests.rs"]
mod prompt_loop_tests;
#[cfg(test)]
#[path = "client/prompt_mapping_tests.rs"]
mod prompt_mapping_tests;
#[cfg(test)]
#[path = "client/protocol_tests.rs"]
mod protocol_tests;
#[cfg(test)]
#[path = "client/runtime_tests.rs"]
mod runtime_tests;
#[cfg(test)]
#[path = "client/session_control_tests.rs"]
mod session_control_tests;
#[cfg(test)]
#[path = "client/session_meta_tests.rs"]
mod session_meta_tests;
#[cfg(test)]
#[path = "client/timeout_messages_tests.rs"]
mod timeout_messages_tests;

use self::helpers::*;
use self::state::*;

const DEFAULT_ACTOR_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone)]
/// ACP 客户端的可复用句柄。
///
/// 该类型是上层与 ACP 代理交互的主要入口。它持有启动配置、权限策略、
/// 回调和后台 actor 状态；克隆句柄会共享同一个 actor 状态和当前提示词控制。
pub struct AcpClient {
    agent_name: String,
    config: AcpAgentConfig,
    client_name: String,
    client_version: String,
    mcp_servers: Vec<acp::McpServer>,
    permission_mode: PermissionMode,
    full_access: bool,
    non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    auth_credentials: HashMap<String, String>,
    auth_policy: AuthPolicy,
    session_options: Option<AcpSessionOptions>,
    verbose: bool,
    on_acp_message: Option<AcpMessageCallback>,
    on_acp_output_message: Option<AcpMessageCallback>,
    on_session_update: Option<SessionUpdateCallback>,
    on_client_operation: Option<ClientOperationCallback>,
    permission_stats: Arc<Mutex<PermissionStats>>,
    active_prompt: Arc<Mutex<Option<ActivePromptControl>>>,
    cancelling_session_ids: Arc<Mutex<HashSet<String>>>,
    actor_state: Arc<Mutex<AcpClientActorState>>,
    actor_idle_timeout: Duration,
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
