//! 通过应用服务器执行 TUI 所有的后台工作区命令。
//!
//! 这是非交互命令在活动工作区运行的 TUI 边界：调用方提供 argv、cwd、环境覆盖、超时和
//! 输出上限，runner 转成应用服务器 `command/exec`。命令不得提示 stdin，元数据刷新通常
//! 必须限制输出；`/diff` 等拥有完整用户可见载荷的调用方可显式取消上限。
//! App-server-backed workspace command execution for TUI-owned background lookups.
//!
//! This module is the TUI boundary for non-interactive commands that need to run wherever
//! the active workspace lives. Callers describe a command in terms of argv, cwd, environment
//! overrides, timeout, and output cap; the runner translates that request to app-server
//! `command/exec`. Keeping this as a TUI-local abstraction lets status surfaces avoid knowing
//! whether the current app-server is embedded or remote.
//!
//! Commands sent through this path should not prompt for stdin. Most callers should keep output
//! bounded so metadata refreshes cannot grow into unbounded background processes; callers that own a
//! full user-visible payload, such as `/diff`, can explicitly opt out of output capping.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use codex_app_server_client::AppServerRequestHandle;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::CommandExecParams;
use codex_app_server_protocol::CommandExecResponse;
use codex_app_server_protocol::RequestId;
use uuid::Uuid;

/// TUI 组件运行工作区命令时共享的句柄。
/// Shared handle for running workspace commands from TUI components.
pub(crate) type WorkspaceCommandRunner = Arc<dyn WorkspaceCommandExecutor>;

/// 描述在活动工作区执行的有界非交互命令；使用 argv 而非 shell，避免调用方引用用户或仓库数据。
/// cwd 由应用服务器按当前会话工作区规则解释，因而同一请求适用于嵌入式和远程实例。
/// Describes a bounded non-interactive command to execute in the active workspace.
///
/// The command is intentionally argv-based rather than shell-based so callers do not need to quote
/// user or repository data. `cwd` is interpreted by app-server relative to the workspace rules for
/// the active session, which is what makes the same request shape work for embedded and remote
/// app-server instances.
#[derive(Clone, Debug)]
pub(crate) struct WorkspaceCommand {
    /// 不经过 shell 插值执行的程序和参数。
    /// Program and arguments to execute without shell interpolation.
    pub(crate) argv: Vec<String>,
    /// 命令工作目录；与应用服务器会话 cwd 相同时可省略。
    /// Working directory for the command, if different from app-server's session cwd.
    pub(crate) cwd: Option<PathBuf>,
    /// 环境变量覆盖，其中 `None` 表示移除变量。
    /// Environment overrides where `None` removes a variable.
    pub(crate) env: HashMap<String, Option<String>>,
    /// 应用服务器取消命令前允许的最大墙钟时间。
    /// Maximum wall-clock duration before app-server cancels the command.
    pub(crate) timeout: Duration,
    /// 应用服务器返回的 stdout/stderr 最大捕获字节数。
    /// Maximum captured stdout/stderr bytes returned by app-server.
    pub(crate) output_bytes_cap: usize,
    /// 应用服务器是否应返回不受上限约束的 stdout/stderr。
    /// Whether app-server should return uncapped stdout/stderr.
    pub(crate) disable_output_cap: bool,
}

impl WorkspaceCommand {
    /// 使用适合元数据探测的保守默认值创建工作区命令。
    /// Creates a workspace command with conservative defaults for metadata probes.
    pub(crate) fn new(argv: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            argv: argv.into_iter().map(Into::into).collect(),
            cwd: None,
            env: HashMap::new(),
            timeout: Duration::from_secs(/*secs*/ 5),
            output_bytes_cap: 64 * 1024,
            disable_output_cap: false,
        }
    }

    /// 设置命令工作目录。
    /// Sets the command working directory.
    pub(crate) fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// 添加或替换一项环境变量覆盖。
    /// Adds or replaces one environment variable override.
    pub(crate) fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), Some(value.into()));
        self
    }

    /// 设置应用服务器取消命令前的最大墙钟时间。
    /// Sets the maximum wall-clock duration before app-server cancels the command.
    pub(crate) fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 请求应用服务器不限制 stdout/stderr 捕获大小。
    /// Requests uncapped stdout/stderr capture from app-server.
    pub(crate) fn disable_output_cap(mut self) -> Self {
        self.disable_output_cap = true;
        self
    }
}

/// 已完成工作区命令的捕获结果。
/// Captured result from a completed workspace command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceCommandOutput {
    /// 应用服务器报告的进程退出状态码。
    /// Process exit status code reported by app-server.
    pub(crate) exit_code: i32,
    /// 应用服务器应用输出上限后捕获的 stdout。
    /// Captured stdout after app-server output capping.
    pub(crate) stdout: String,
    /// 应用服务器应用输出上限后捕获的 stderr。
    /// Captured stderr after app-server output capping.
    pub(crate) stderr: String,
}

impl WorkspaceCommandOutput {
    /// 返回进程是否成功退出。
    /// Returns whether the process exited successfully.
    pub(crate) fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// 命令结果可用前发生的传输或协议失败；非零进程退出仍用 `WorkspaceCommandOutput` 表示，
/// 使调用方能区分普通探测未命中与应用服务器请求失败。
/// Transport or protocol failure before a command result was available.
///
/// Non-zero process exits are represented as `WorkspaceCommandOutput` so callers can distinguish
/// a normal probe miss from an app-server request failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkspaceCommandError {
    message: String,
}

impl WorkspaceCommandError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for WorkspaceCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for WorkspaceCommandError {}

/// 通过活动 TUI 应用服务器会话执行非交互工作区命令；实现决定工作区位置，
/// 调用方不应按本地或远程执行分支。
/// Executes non-interactive workspace commands through the active TUI app-server session.
///
/// Implementations decide where the workspace lives. Callers provide argv/cwd/env and should not
/// branch on local versus remote execution.
pub(crate) trait WorkspaceCommandExecutor: Send + Sync {
    /// 运行工作区命令并返回捕获输出或应用服务器请求错误；错误是基础设施失败，
    /// 成功输出中的非零退出码是普通命令失败，装箱 future 保持 trait 对象安全。
    /// Runs a workspace command and returns captured output or an app-server request error.
    ///
    /// Callers should treat errors as infrastructure failures and should treat successful output
    /// with a non-zero exit code as ordinary command failure. Returning a boxed future keeps the
    /// trait object-safe.
    fn run(
        &self,
        command: WorkspaceCommand,
    ) -> Pin<
        Box<dyn Future<Output = Result<WorkspaceCommandOutput, WorkspaceCommandError>> + Send + '_>,
    >;
}

/// 将每个请求转发到活动应用服务器的工作区命令 runner。
/// Workspace command runner that forwards every request to the active app-server.
#[derive(Clone)]
pub(crate) struct AppServerWorkspaceCommandRunner {
    request_handle: AppServerRequestHandle,
}

impl AppServerWorkspaceCommandRunner {
    /// 从当前 TUI 会话持有的应用服务器请求句柄创建 runner。
    /// Creates a runner from an app-server request handle owned by the current TUI session.
    pub(crate) fn new(request_handle: AppServerRequestHandle) -> Self {
        Self { request_handle }
    }
}

impl WorkspaceCommandExecutor for AppServerWorkspaceCommandRunner {
    /// 将命令作为一次性应用服务器 `command/exec` 请求发送；请求非 TTY、不流式传输 stdio，
    /// 使用调用方超时和输出上限，并由应用服务器选择沙箱和权限配置。
    /// Sends the command as a one-off app-server `command/exec` request.
    ///
    /// The request is non-tty, does not stream stdin/stdout/stderr, and uses the caller's timeout
    /// and output cap. It leaves sandbox and permission profile selection to app-server so the same
    /// runner follows the active session's embedded or remote execution policy.
    fn run(
        &self,
        command: WorkspaceCommand,
    ) -> Pin<
        Box<dyn Future<Output = Result<WorkspaceCommandOutput, WorkspaceCommandError>> + Send + '_>,
    > {
        Box::pin(async move {
            let timeout_ms = i64::try_from(command.timeout.as_millis()).unwrap_or(i64::MAX);
            let env = if command.env.is_empty() {
                None
            } else {
                Some(command.env)
            };
            let response: CommandExecResponse = self
                .request_handle
                .request_typed(ClientRequest::OneOffCommandExec {
                    request_id: RequestId::String(format!("workspace-command-{}", Uuid::new_v4())),
                    params: CommandExecParams {
                        command: command.argv,
                        process_id: None,
                        tty: false,
                        stream_stdin: false,
                        stream_stdout_stderr: false,
                        output_bytes_cap: (!command.disable_output_cap)
                            .then_some(command.output_bytes_cap),
                        disable_output_cap: command.disable_output_cap,
                        disable_timeout: false,
                        timeout_ms: Some(timeout_ms),
                        cwd: command.cwd,
                        env,
                        size: None,
                        sandbox_policy: None,
                        permission_profile: None,
                    },
                })
                .await
                .map_err(|err| WorkspaceCommandError::new(err.to_string()))?;

            Ok(WorkspaceCommandOutput {
                exit_code: response.exit_code,
                stdout: response.stdout,
                stderr: response.stderr,
            })
        })
    }
}
