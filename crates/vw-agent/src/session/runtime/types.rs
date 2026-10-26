//! 会话生命周期边界使用的强类型输入、快照与错误。

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};

use tokio_util::sync::CancellationToken;
use vw_api_types::id::TenantScope;

use crate::app::agent::tools::Tool;
use crate::permission::next as permission_next;
use crate::session::processor::{CompactionPhase, ToolEvent};
use crate::session::ui_types::TokenUsage;
use crate::session::{message, session};

/// 只属于单个原生会话的运行时工具集合。
#[derive(Clone, Default)]
pub struct SessionToolset {
    tools: Arc<Vec<Arc<dyn Tool>>>,
}

impl fmt::Debug for SessionToolset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("SessionToolset").field("len", &self.tools.len()).finish()
    }
}

impl SessionToolset {
    /// 从已完成启动和发现的工具构造会话级集合。
    pub(crate) fn new(tools: Vec<Arc<dyn Tool>>) -> Self {
        Self { tools: Arc::new(tools) }
    }

    /// 返回本会话的运行时工具。
    pub(crate) fn tools(&self) -> &[Arc<dyn Tool>] {
        self.tools.as_slice()
    }

    /// 返回会话级工具数量。
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// 返回集合是否为空。
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// 已验证的原生会话访问作用域。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionScope {
    tenant: TenantScope,
    directory: PathBuf,
}

impl SessionScope {
    /// 从租户和现有绝对目录构造作用域，并保存目录的规范化形式。
    pub fn new(tenant: TenantScope, directory: impl AsRef<Path>) -> Result<Self, LifecycleError> {
        let directory = directory.as_ref().to_path_buf();
        if !directory.is_absolute() {
            return Err(LifecycleError::DirectoryMustBeAbsolute { directory });
        }

        let canonical = std::fs::canonicalize(&directory).map_err(|source| {
            LifecycleError::DirectoryUnavailable { directory: directory.clone(), source }
        })?;
        let metadata = std::fs::metadata(&canonical).map_err(|source| {
            LifecycleError::DirectoryUnavailable { directory: canonical.clone(), source }
        })?;
        if !metadata.is_dir() {
            return Err(LifecycleError::DirectoryNotDirectory { directory: canonical });
        }

        Ok(Self { tenant, directory: canonical })
    }

    /// 返回当前作用域的租户。
    pub fn tenant(&self) -> &TenantScope {
        &self.tenant
    }

    /// 返回规范化后的绝对工作目录。
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

/// 创建原生会话所需的完整输入。
#[derive(Debug, Clone)]
pub struct CreateSessionInput {
    /// 已验证的租户和目录作用域。
    pub scope: SessionScope,
    /// 可选父会话 ID；父会话必须属于同一作用域。
    pub parent_id: Option<String>,
    /// 可选标题；为空时沿用原生默认标题。
    pub title: Option<String>,
    /// 可选原生权限规则。
    pub permission: Option<permission_next::Ruleset>,
}

/// 已完成归属校验的原生会话快照。
#[derive(Debug, Clone)]
pub struct SessionSnapshot {
    /// 原生会话元数据，保留原始 ID、slug 与项目 ID。
    pub info: session::Info,
    /// 校验归属后读取的完整原生消息历史。
    pub history: Vec<message::WithParts>,
}

/// 单轮提示结束原因。
#[derive(Debug, Clone)]
pub enum PromptTermination {
    /// 处理器正常完成并返回总令牌用量。
    Completed(TokenUsage),
    /// 调用方显式取消了活动轮次。
    Cancelled,
    /// 处理器以错误终止。
    Failed(String),
}

/// 共享提示运行时输出的协议中立事件。
#[derive(Debug, Clone)]
pub enum PromptEvent {
    /// 仅供 Gateway 暂存的原始 LLM 工件。
    LlmRawArtifact(crate::session::llm_raw::LlmRawArtifact),

    /// 实际加载的上下文文件。
    ContextLoaded {
        /// 文件路径列表。
        paths: Vec<String>,
    },
    /// 可恢复模型故障的非终止状态。
    ModelRecovery(crate::session::processor::ModelRecovery),
    /// 助手文本增量。
    Text(String),
    /// 已净化的类型化工具事件。
    Tool(ToolEvent),
    /// 自动压缩生命周期事件。
    Compaction {
        /// 当前压缩阶段。
        phase: CompactionPhase,
    },
    /// 处理器步骤开始。
    StepStart {
        /// 步骤索引。
        step_index: u32,
        /// 创建时间。
        created_ms: u64,
        /// 模型。
        model: Option<String>,
    },
    /// 处理器步骤完成。
    StepFinish {
        /// 步骤索引。
        step_index: u32,
        /// 完成时间。
        finished_ms: u64,
        /// 用量。
        usage: TokenUsage,
        /// 完成原因。
        finish_reason: Option<String>,
        /// 模型。
        model: Option<String>,
    },
    /// 工具轮次结束。
    PostToolRound {
        /// 步骤索引。
        step_index: u32,
    },
    /// 单轮唯一终态。
    Terminated(PromptTermination),
}

/// 显式、幂等的提示取消句柄。
#[derive(Debug, Clone)]
pub struct PromptCancelHandle {
    pub(crate) explicit: Arc<AtomicBool>,
    pub(crate) shell_cutoff: Arc<std::sync::Mutex<Option<std::time::Instant>>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) shell_scope: Option<crate::tools::shell::processes::ShellProcessScope>,
    pub(crate) token: CancellationToken,
    pub(crate) terminal: Arc<AtomicBool>,
    pub(crate) events: mpsc::Sender<PromptEvent>,
}

impl PromptCancelHandle {
    /// 取消活动轮次；仅首次成功取消返回 `true`。
    pub fn cancel(&self) -> bool {
        self.explicit.store(true, Ordering::Release);
        self.token.cancel();
        let cleanup = self.reap_shell_processes();
        if self.terminal.swap(true, Ordering::AcqRel) {
            return false;
        }
        let termination = match cleanup {
            Ok(()) => PromptTermination::Cancelled,
            Err(error) => PromptTermination::Failed(error.to_string()),
        };
        let _ = self.events.send(PromptEvent::Terminated(termination));
        true
    }

    /// Stop consuming this prompt without treating transport loss as user stop.
    pub(crate) fn interrupt(&self) {
        self.token.cancel();
    }

    /// A retained stop handle must not cancel commands from a later prompt.
    pub(crate) fn finish_shell_range(&self) {
        self.shell_cutoff
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_or_insert_with(std::time::Instant::now);
    }

    pub(crate) fn reap_shell_processes(&self) -> anyhow::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(scope) = &self.shell_scope
            && let Some(host) = crate::tools::shell::processes::ShellProcesses::existing()
        {
            let cutoff =
                *self.shell_cutoff.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            host.cancel_scope_before(scope, cutoff)?;
        }
        Ok(())
    }

    /// 返回取消是否已被请求。
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

/// 已启动提示轮次的事件接收端和取消句柄。
#[derive(Debug)]
pub struct PromptTurn {
    /// 按产生顺序接收事件，且最多接收一个终态。
    pub events: mpsc::Receiver<PromptEvent>,
    /// 可跨线程调用的幂等取消句柄。
    pub cancel: PromptCancelHandle,
}

/// 会话生命周期边界的分类错误。
#[derive(Debug)]
pub enum LifecycleError {
    /// 调用方传入了相对目录。
    DirectoryMustBeAbsolute {
        /// 被拒绝的目录。
        directory: PathBuf,
    },
    /// 目录不存在、不可访问或无法规范化。
    DirectoryUnavailable {
        /// 被拒绝的目录。
        directory: PathBuf,
        /// 文件系统返回的具体原因。
        source: io::Error,
    },
    /// 路径存在但不是目录。
    DirectoryNotDirectory {
        /// 被拒绝的路径。
        directory: PathBuf,
    },
    /// 项目发现或项目元数据持久化失败。
    Project(crate::project::Error),
    /// 原生会话操作失败。
    Session(session::Error),
    /// 当前租户中不存在目标会话。
    SessionNotFound {
        /// 未找到的会话 ID。
        session_id: String,
    },
    /// 会话目录或项目不属于请求的作用域。
    SessionOwnershipMismatch {
        /// 被拒绝的会话 ID。
        session_id: String,
        /// 请求作用域的规范化目录。
        expected_directory: PathBuf,
        /// 会话记录中的目录。
        actual_directory: PathBuf,
        /// 请求作用域对应的项目 ID。
        expected_project_id: String,
        /// 会话记录中的项目 ID。
        actual_project_id: String,
    },
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirectoryMustBeAbsolute { directory } => {
                write!(f, "session directory must be absolute: {}", directory.display())
            }
            Self::DirectoryUnavailable { directory, source } => {
                write!(f, "session directory is unavailable ({}): {source}", directory.display())
            }
            Self::DirectoryNotDirectory { directory } => {
                write!(f, "session directory is not a directory: {}", directory.display())
            }
            Self::Project(error) => write!(f, "session project resolution failed: {error}"),
            Self::Session(error) => write!(f, "native session operation failed: {error}"),
            Self::SessionNotFound { session_id } => {
                write!(f, "session not found in the requested tenant scope: {session_id}")
            }
            Self::SessionOwnershipMismatch {
                session_id,
                expected_directory,
                actual_directory,
                expected_project_id,
                actual_project_id,
            } => write!(
                f,
                "session {session_id} does not belong to directory {} and project {expected_project_id} (recorded directory: {}, project: {actual_project_id})",
                expected_directory.display(),
                actual_directory.display()
            ),
        }
    }
}

impl std::error::Error for LifecycleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DirectoryUnavailable { source, .. } => Some(source),
            Self::Project(error) => Some(error),
            Self::Session(error) => Some(error),
            _ => None,
        }
    }
}

impl From<crate::project::Error> for LifecycleError {
    fn from(value: crate::project::Error) -> Self {
        Self::Project(value)
    }
}

impl From<session::Error> for LifecycleError {
    fn from(value: session::Error) -> Self {
        Self::Session(value)
    }
}
