//! ACP 客户端内部状态类型。
//!
//! 这些类型只服务于 `client/*` 子模块，不构成 crate 公共 API。

use super::*;

#[derive(Debug)]
pub(super) enum InternalEvent {
    Delta(String),
    SessionChanged { expected: String, actual: String },
}

#[derive(Debug, Clone)]
pub(super) struct AuthSelection {
    pub(super) method_id: String,
    pub(super) source: &'static str,
}

#[derive(Clone)]
pub(super) struct ActivePromptControl {
    pub(super) session_id: String,
    pub(super) cancel_tx: watch::Sender<bool>,
    pub(super) completed_rx: watch::Receiver<bool>,
}

pub(super) struct AcpEventClient {
    pub(super) claude_permissions: bool,
    pub(super) transient_permissions: bool,
    pub(super) reject_untyped_workflow_permissions: bool,
    pub(super) expected_session_id: Arc<Mutex<Option<String>>>,
    pub(super) event_tx: mpsc::UnboundedSender<InternalEvent>,
    pub(super) filesystem: FileSystemHandlers,
    pub(super) terminal_manager: TerminalManager,
    pub(super) permission_mode: PermissionMode,
    pub(super) non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub(super) on_session_update: Option<SessionUpdateCallback>,
    pub(super) permission_stats: Arc<Mutex<PermissionStats>>,
    pub(super) cancelling_session_ids: Arc<Mutex<HashSet<String>>>,
}

/// 已启动 ACP 代理进程的基础句柄集合。
///
/// `child` 保留子进程控制权，`stderr_task` 异步收集标准错误输出，用于进程
/// 初始化或会话创建失败时补充诊断上下文。
pub(crate) struct ProcessHandles {
    pub(super) child: Child,
    pub(super) stderr_task: Option<tokio::task::JoinHandle<String>>,
}

#[derive(Default)]
pub(super) struct AcpClientActorState {
    pub(super) handle: Option<AcpClientActorHandle>,
    pub(super) reusable_session_id: Option<String>,
    pub(super) lifecycle: AgentLifecycleSnapshot,
    pub(super) session_capabilities: Option<crate::SessionCapabilities>,
    pub(super) qwen_runtime_verified: bool,
    pub(super) qwen_confirmed_session: Option<String>,
    pub(super) claude_runtime_verified: bool,
    pub(super) claude_confirmed_session: Option<String>,
    pub(super) codex_runtime_verified: bool,
    pub(super) codex_confirmed_session: Option<String>,
    // Acknowledged iFlow control intent, separate from protocol mode readback.
    pub(super) iflow_acknowledged_mode: Option<(String, String)>,
    pub(super) pending_controls: HashMap<(String, String), String>,
}

pub(super) struct AcpClientActorHandle {
    pub(super) command_tx: mpsc::UnboundedSender<ActorCommand>,
    pub(super) thread: Option<thread::JoinHandle<()>>,
}

pub(super) struct ActorRuntime {
    pub(super) cwd: PathBuf,
    pub(super) child: Child,
    pub(super) stderr_task: Option<tokio::task::JoinHandle<String>>,
    pub(super) expected_session_id: Arc<Mutex<Option<String>>>,
    pub(super) event_rx: mpsc::UnboundedReceiver<InternalEvent>,
    pub(super) conn: acp::ClientSideConnection,
    pub(super) io_closed_rx: oneshot::Receiver<()>,
    pub(super) io_task: tokio::task::JoinHandle<()>,
}

pub(super) enum ActorCommand {
    CreateSession {
        cwd: PathBuf,
        response_tx: oneshot::Sender<Result<SessionInfo, AcpError>>,
    },
    LoadSession {
        session_id: String,
        cwd: PathBuf,
        response_tx: oneshot::Sender<Result<SessionInfo, AcpError>>,
    },
    ResumeSession {
        session_id: String,
        cwd: PathBuf,
        response_tx: oneshot::Sender<Result<SessionInfo, AcpError>>,
    },
    SetSessionMode {
        session_id: String,
        cwd: PathBuf,
        mode_id: String,
        response_tx: oneshot::Sender<Result<(), AcpError>>,
    },
    SetSessionConfigOption {
        session_id: String,
        cwd: PathBuf,
        option_name: String,
        value_id: String,
        response_tx: oneshot::Sender<Result<acp::SetSessionConfigOptionResponse, AcpError>>,
    },
    SetSessionModel {
        session_id: String,
        cwd: PathBuf,
        model: String,
        response_tx: oneshot::Sender<Result<(), AcpError>>,
    },
    RunPrompt {
        request: PromptRequest,
        event_tx: mpsc::UnboundedSender<PromptEvent>,
        response_tx: oneshot::Sender<Result<PromptResult, AcpError>>,
    },
    Close {
        response_tx: oneshot::Sender<()>,
    },
}

#[derive(Clone, Default)]
pub(super) struct ChildExitSummary {
    pub(super) exit_code: Option<i32>,
    pub(super) signal: Option<String>,
}

#[derive(Default)]
/// 已结束代理进程的收尾结果。
///
/// 该结构将退出状态和 stderr 输出分开保存，便于调用方按错误类型决定是否
/// 将进程上下文合并到用户可见错误里。
pub(crate) struct FinalizedChild {
    pub(super) summary: ChildExitSummary,
    pub(super) stderr_output: String,
}

#[derive(Clone)]
pub(super) struct MessageTap {
    pub(super) direction: AcpMessageDirection,
    pub(super) on_message: Option<AcpMessageCallback>,
    pub(super) on_output_message: Option<AcpMessageCallback>,
    pub(super) buffer: Vec<u8>,
}

pub(super) struct TappedReader<R> {
    pub(super) inner: R,
    pub(super) tap: MessageTap,
}

pub(super) struct TappedWriter<W> {
    pub(super) inner: W,
    pub(super) tap: MessageTap,
}
