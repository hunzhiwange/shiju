//! 终端工具请求的执行与输出回传逻辑。
//!
//! 本模块封装终端命令相关工具能力，包括启动命令、跟踪输出、等待完成、
//! 停止进程以及查询历史输出等操作。
//!
//! 它承担的是工具协议与本地进程管理之间的适配层职责，
//! 需要同时兼顾输出缓存、生命周期控制和错误转换。

use std::collections::HashMap;
use std::future::Future;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use agent_client_protocol::{
    CreateTerminalRequest, CreateTerminalResponse, KillTerminalRequest, KillTerminalResponse,
    ReleaseTerminalRequest, ReleaseTerminalResponse, TerminalOutputRequest, TerminalOutputResponse,
    WaitForTerminalExitRequest, WaitForTerminalExitResponse,
};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{Map, Value, json};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Child;
use tokio::sync::{Notify, mpsc, oneshot};

use crate::errors::{
    AcpxErrorOptions, ErrorSource, PermissionDeniedError, PermissionPromptUnavailableError,
};
use crate::permission_prompt::{
    PermissionPromptOptions, can_prompt_for_permission, prompt_for_permission,
};
use crate::spawn_command_options::build_spawn_command;
use crate::types::{
    ClientOperation, ClientOperationMethod, ClientOperationStatus, NonInteractivePermissionPolicy,
    OutputErrorCode, OutputErrorOrigin, PermissionMode,
};

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod terminal_tests;

const DEFAULT_TERMINAL_OUTPUT_LIMIT_BYTES: usize = 64 * 1024;
const DEFAULT_KILL_GRACE_MS: u64 = 1_500;

static NEXT_TERMINAL_ID: AtomicU64 = AtomicU64::new(1);

pub type TerminalConfirmExecuteFuture =
    Pin<Box<dyn Future<Output = Result<bool, ErrorSource>> + Send + 'static>>;
pub type TerminalConfirmExecuteFn =
    Arc<dyn Fn(String) -> TerminalConfirmExecuteFuture + Send + Sync>;
pub type TerminalOperationCallback = Arc<dyn Fn(ClientOperation) + Send + Sync>;

#[derive(Clone, Default)]
pub struct TerminalManagerOptions {
    pub cwd: PathBuf,
    pub permission_mode: PermissionMode,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub on_operation: Option<TerminalOperationCallback>,
    pub confirm_execute: Option<TerminalConfirmExecuteFn>,
    pub kill_grace_ms: Option<u64>,
}

pub struct TerminalManager {
    cwd: PathBuf,
    permission_mode: PermissionMode,
    non_interactive_permissions: NonInteractivePermissionPolicy,
    on_operation: Option<TerminalOperationCallback>,
    uses_default_confirm_execute: bool,
    confirm_execute: TerminalConfirmExecuteFn,
    kill_grace_ms: u64,
    terminals: Mutex<HashMap<String, Arc<ManagedTerminal>>>,
}

struct ManagedTerminal {
    state: Mutex<TerminalState>,
    exit_notify: Notify,
    command_tx: mpsc::UnboundedSender<TerminalCommand>,
}

struct TerminalState {
    output: Vec<u8>,
    truncated: bool,
    output_byte_limit: usize,
    exit_status: Option<TerminalExitState>,
}

#[derive(Clone)]
struct TerminalExitState {
    exit_code: Option<i32>,
    signal: Option<String>,
}

enum TerminalCommand {
    Kill { done_tx: oneshot::Sender<()> },
}

impl TerminalManager {
    pub fn new(options: TerminalManagerOptions) -> Self {
        let uses_default_confirm_execute = options.confirm_execute.is_none();
        Self {
            cwd: options.cwd,
            permission_mode: options.permission_mode,
            non_interactive_permissions: options
                .non_interactive_permissions
                .unwrap_or(NonInteractivePermissionPolicy::Deny),
            on_operation: options.on_operation,
            uses_default_confirm_execute,
            confirm_execute: options
                .confirm_execute
                .unwrap_or_else(|| Arc::new(default_confirm_execute)),
            kill_grace_ms: options.kill_grace_ms.unwrap_or(DEFAULT_KILL_GRACE_MS),
            terminals: Mutex::new(HashMap::new()),
        }
    }

    pub fn update_permission_policy(
        &mut self,
        permission_mode: PermissionMode,
        non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    ) {
        self.permission_mode = permission_mode;
        self.non_interactive_permissions =
            non_interactive_permissions.unwrap_or(NonInteractivePermissionPolicy::Deny);
    }

    pub async fn create_terminal(
        &self,
        params: &CreateTerminalRequest,
    ) -> Result<CreateTerminalResponse, ErrorSource> {
        let record = request_record(params)?;
        let command = required_string(&record, "command")?;
        let args = string_array(&record, "args");
        let cwd = terminal_cwd(&record, &self.cwd)?;
        let env = env_overrides(&record);
        let output_byte_limit = output_byte_limit(&record);
        let command_line = to_command_line(&command, &args);
        let summary = format!("terminal/create: {command_line}");

        self.emit_operation(ClientOperation {
            method: ClientOperationMethod::TerminalCreate,
            status: ClientOperationStatus::Running,
            summary: summary.clone(),
            details: None,
            timestamp: now_iso(),
        });

        let result = async {
            if !self.is_execute_approved(&cwd, &command_line).await? {
                return Err(Box::new(permission_denied_error(
                    "Permission denied for terminal/create",
                )) as ErrorSource);
            }

            let mut child = build_spawn_command(command.clone(), &env);
            child
                .args(&args)
                .current_dir(&cwd)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);

            let mut child = child.spawn()?;
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            let (command_tx, command_rx) = mpsc::unbounded_channel();
            let terminal = Arc::new(ManagedTerminal::new(output_byte_limit, command_tx));
            if let Some(stdout) = stdout {
                spawn_output_task(stdout, terminal.clone());
            }
            if let Some(stderr) = stderr {
                spawn_output_task(stderr, terminal.clone());
            }

            let terminal_id = next_terminal_id();
            self.terminals.lock().insert(terminal_id.clone(), terminal.clone());
            tokio::spawn(run_terminal_process(child, terminal, command_rx, self.kill_grace_ms));

            Ok(create_terminal_response(terminal_id))
        }
        .await;

        self.emit_completion(
            ClientOperationMethod::TerminalCreate,
            summary,
            result
                .as_ref()
                .ok()
                .map(|response| terminal_id_from_response(response).unwrap_or_default())
                .filter(|terminal_id| !terminal_id.is_empty())
                .map(|terminal_id| format!("terminalId={terminal_id}")),
            result.as_ref().err().map(ToString::to_string),
        );
        result
    }

    pub async fn terminal_output(
        &self,
        params: &TerminalOutputRequest,
    ) -> Result<TerminalOutputResponse, ErrorSource> {
        let terminal_id = terminal_id_from_request(params)?;
        let terminal =
            self.get_terminal(&terminal_id).ok_or_else(|| unknown_terminal_error(&terminal_id))?;
        let snapshot = terminal.snapshot();

        self.emit_operation(ClientOperation {
            method: ClientOperationMethod::TerminalOutput,
            status: ClientOperationStatus::Completed,
            summary: format!("terminal/output: {terminal_id}"),
            details: None,
            timestamp: now_iso(),
        });

        Ok(terminal_output_response(
            String::from_utf8_lossy(&snapshot.output).into_owned(),
            snapshot.truncated,
            snapshot.exit_status,
        ))
    }

    pub async fn wait_for_terminal_exit(
        &self,
        params: &WaitForTerminalExitRequest,
    ) -> Result<WaitForTerminalExitResponse, ErrorSource> {
        let terminal_id = terminal_id_from_request(params)?;
        let terminal =
            self.get_terminal(&terminal_id).ok_or_else(|| unknown_terminal_error(&terminal_id))?;
        let response = wait_for_exit_response(&terminal).await;

        self.emit_operation(ClientOperation {
            method: ClientOperationMethod::TerminalWaitForExit,
            status: ClientOperationStatus::Completed,
            summary: format!("terminal/wait_for_exit: {terminal_id}"),
            details: Some(format!(
                "exitCode={}, signal={}",
                response_record_number_or_null(&response, "exitCode"),
                response_record_string_or_null(&response, "signal"),
            )),
            timestamp: now_iso(),
        });

        Ok(response)
    }

    pub async fn kill_terminal(
        &self,
        params: &KillTerminalRequest,
    ) -> Result<KillTerminalResponse, ErrorSource> {
        let terminal_id = terminal_id_from_request(params)?;
        let terminal =
            self.get_terminal(&terminal_id).ok_or_else(|| unknown_terminal_error(&terminal_id))?;
        let summary = format!("terminal/kill: {terminal_id}");

        self.emit_operation(ClientOperation {
            method: ClientOperationMethod::TerminalKill,
            status: ClientOperationStatus::Running,
            summary: summary.clone(),
            details: None,
            timestamp: now_iso(),
        });

        let result = self.kill_process(&terminal).await.map(|()| empty_kill_terminal_response());
        self.emit_completion(
            ClientOperationMethod::TerminalKill,
            summary,
            None,
            result.as_ref().err().map(ToString::to_string),
        );
        result
    }

    pub async fn release_terminal(
        &self,
        params: &ReleaseTerminalRequest,
    ) -> Result<ReleaseTerminalResponse, ErrorSource> {
        let terminal_id = terminal_id_from_request(params)?;
        let summary = format!("terminal/release: {terminal_id}");

        self.emit_operation(ClientOperation {
            method: ClientOperationMethod::TerminalRelease,
            status: ClientOperationStatus::Running,
            summary: summary.clone(),
            details: None,
            timestamp: now_iso(),
        });

        let Some(terminal) = self.get_terminal(&terminal_id) else {
            self.emit_completion(
                ClientOperationMethod::TerminalRelease,
                summary,
                Some("already released".to_string()),
                None,
            );
            return Ok(empty_release_terminal_response());
        };

        let result = async {
            self.kill_process(&terminal).await?;
            let _ = wait_for_exit_response(&terminal).await;
            terminal.clear_output();
            self.terminals.lock().remove(&terminal_id);
            Ok(empty_release_terminal_response())
        }
        .await;

        self.emit_completion(
            ClientOperationMethod::TerminalRelease,
            summary,
            None,
            result.as_ref().err().map(ToString::to_string),
        );
        result
    }

    pub async fn shutdown(&self) {
        let terminal_ids = self.terminals.lock().keys().cloned().collect::<Vec<_>>();
        for terminal_id in terminal_ids {
            let params = empty_release_request(&terminal_id);
            let _ = self.release_terminal(&params).await;
        }
    }

    fn get_terminal(&self, terminal_id: &str) -> Option<Arc<ManagedTerminal>> {
        self.terminals.lock().get(terminal_id).cloned()
    }

    fn emit_operation(&self, operation: ClientOperation) {
        if let Some(on_operation) = &self.on_operation {
            on_operation(operation);
        }
    }

    fn emit_completion(
        &self,
        method: ClientOperationMethod,
        summary: String,
        success_details: Option<String>,
        error: Option<String>,
    ) {
        self.emit_operation(ClientOperation {
            method,
            status: if error.is_some() {
                ClientOperationStatus::Failed
            } else {
                ClientOperationStatus::Completed
            },
            summary,
            details: error.or(success_details),
            timestamp: now_iso(),
        });
    }

    async fn is_execute_approved(
        &self,
        cwd: &Path,
        command_line: &str,
    ) -> Result<bool, ErrorSource> {
        match self.permission_mode {
            PermissionMode::ApproveAll => Ok(true),
            PermissionMode::DenyAll => Ok(false),
            PermissionMode::ApproveReads => {
                if self.uses_default_confirm_execute && is_within_root(&self.cwd, cwd) {
                    return Ok(true);
                }
                if self.uses_default_confirm_execute
                    && self.non_interactive_permissions == NonInteractivePermissionPolicy::Fail
                    && !can_prompt_for_permission()
                {
                    return Err(Box::new(PermissionPromptUnavailableError::new()));
                }
                (self.confirm_execute)(command_line.to_string()).await
            }
        }
    }

    async fn kill_process(&self, terminal: &Arc<ManagedTerminal>) -> Result<(), ErrorSource> {
        if terminal.has_exited() {
            return Ok(());
        }

        let (done_tx, done_rx) = oneshot::channel();
        if terminal.command_tx.send(TerminalCommand::Kill { done_tx }).is_err() {
            return if terminal.has_exited() {
                Ok(())
            } else {
                Err("terminal process controller unavailable".into())
            };
        }

        let _ = done_rx.await;
        Ok(())
    }
}

impl ManagedTerminal {
    fn new(output_byte_limit: usize, command_tx: mpsc::UnboundedSender<TerminalCommand>) -> Self {
        Self {
            state: Mutex::new(TerminalState {
                output: Vec::new(),
                truncated: false,
                output_byte_limit,
                exit_status: None,
            }),
            exit_notify: Notify::new(),
            command_tx,
        }
    }

    fn append_output(&self, chunk: &[u8]) {
        if chunk.is_empty() {
            return;
        }

        let mut state = self.state.lock();
        state.output.extend_from_slice(chunk);
        if state.output.len() > state.output_byte_limit {
            state.output = trim_to_utf8_boundary(&state.output, state.output_byte_limit);
            state.truncated = true;
        }
    }

    fn record_exit(&self, status: ExitStatus) {
        {
            let mut state = self.state.lock();
            if state.exit_status.is_some() {
                return;
            }
            state.exit_status = Some(TerminalExitState {
                exit_code: status.code(),
                signal: exit_signal_name(&status),
            });
        }
        self.exit_notify.notify_waiters();
    }

    fn snapshot(&self) -> TerminalSnapshot {
        let state = self.state.lock();
        TerminalSnapshot {
            output: state.output.clone(),
            truncated: state.truncated,
            exit_status: state.exit_status.clone(),
        }
    }

    fn has_exited(&self) -> bool {
        self.state.lock().exit_status.is_some()
    }

    fn clear_output(&self) {
        self.state.lock().output.clear();
    }
}

struct TerminalSnapshot {
    output: Vec<u8>,
    truncated: bool,
    exit_status: Option<TerminalExitState>,
}

fn default_confirm_execute(command_line: String) -> TerminalConfirmExecuteFuture {
    Box::pin(async move {
        prompt_for_permission(&PermissionPromptOptions {
            prompt: format!("\n[permission] Allow terminal command \"{command_line}\"? (y/N) "),
            header: None,
            details: None,
        })
        .map_err(|error| Box::new(error) as ErrorSource)
    })
}

async fn run_terminal_process(
    mut child: Child,
    terminal: Arc<ManagedTerminal>,
    mut command_rx: mpsc::UnboundedReceiver<TerminalCommand>,
    kill_grace_ms: u64,
) {
    while !terminal.has_exited() {
        tokio::select! {
            status = child.wait() => {
                terminal.record_exit(status.unwrap_or_else(|_| exit_status_from_failed_wait()));
                break;
            }
            maybe_command = command_rx.recv() => {
                match maybe_command {
                    Some(TerminalCommand::Kill { done_tx }) => {
                        kill_child_process(&mut child, &terminal, kill_grace_ms).await;
                        let _ = done_tx.send(());
                    }
                    None => break,
                }
            }
        }
    }
}

fn spawn_output_task<R>(mut reader: R, terminal: Arc<ManagedTerminal>)
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut buffer = vec![0_u8; 8 * 1024];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => break,
                Ok(read) => terminal.append_output(&buffer[..read]),
                Err(_) => break,
            }
        }
    });
}

async fn kill_child_process(
    child: &mut Child,
    terminal: &Arc<ManagedTerminal>,
    kill_grace_ms: u64,
) {
    if terminal.has_exited() {
        return;
    }

    if !send_terminate_signal(child) {
        return;
    }

    let exited_after_term = tokio::select! {
        status = child.wait() => {
            if let Ok(status) = status {
                terminal.record_exit(status);
            }
            true
        }
        _ = tokio::time::sleep(Duration::from_millis(kill_grace_ms)) => false,
    };

    if exited_after_term || terminal.has_exited() {
        return;
    }

    if !send_kill_signal(child) {
        return;
    }

    if let Ok(Ok(status)) =
        tokio::time::timeout(Duration::from_millis(kill_grace_ms), child.wait()).await
    {
        terminal.record_exit(status);
    }
}

#[cfg(unix)]
fn send_terminate_signal(child: &Child) -> bool {
    send_signal(child, libc::SIGTERM)
}

#[cfg(not(unix))]
fn send_terminate_signal(child: &mut Child) -> bool {
    child.start_kill().is_ok()
}

#[cfg(unix)]
fn send_kill_signal(child: &Child) -> bool {
    send_signal(child, libc::SIGKILL)
}

#[cfg(not(unix))]
fn send_kill_signal(child: &mut Child) -> bool {
    child.start_kill().is_ok()
}

#[cfg(unix)]
fn send_signal(child: &Child, signal: i32) -> bool {
    let Some(pid) = child.id() else {
        return false;
    };
    unsafe { libc::kill(pid as i32, signal) == 0 }
}

fn trim_to_utf8_boundary(buffer: &[u8], limit: usize) -> Vec<u8> {
    if limit == 0 {
        return Vec::new();
    }
    if buffer.len() <= limit {
        return buffer.to_vec();
    }

    let mut start = buffer.len() - limit;
    while start < buffer.len() && (buffer[start] & 0b1100_0000) == 0b1000_0000 {
        start += 1;
    }
    if start >= buffer.len() {
        start = buffer.len() - limit;
    }
    buffer[start..].to_vec()
}

async fn wait_for_exit_response(terminal: &Arc<ManagedTerminal>) -> WaitForTerminalExitResponse {
    loop {
        if let Some(exit_status) = terminal.state.lock().exit_status.clone() {
            return wait_for_terminal_exit_response(exit_status);
        }
        terminal.exit_notify.notified().await;
    }
}

fn to_command_line(command: &str, args: &[String]) -> String {
    let rendered_args = args.iter().map(|arg| format!("{arg:?}")).collect::<Vec<_>>().join(" ");
    if rendered_args.is_empty() {
        command.to_string()
    } else {
        format!("{command} {rendered_args}")
    }
}

fn terminal_cwd(record: &Map<String, Value>, default_cwd: &Path) -> Result<PathBuf, ErrorSource> {
    let cwd = record
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| default_cwd.to_path_buf());
    if !cwd.is_absolute() {
        return Err(format!("cwd must be absolute: {}", cwd.display()).into());
    }
    Ok(cwd)
}

fn normalize_absolute_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(value) => normalized.push(value),
        }
    }
    normalized
}

fn is_within_root(root_dir: &Path, target_path: &Path) -> bool {
    let root_dir = normalize_absolute_path(root_dir);
    let target_path = normalize_absolute_path(target_path);
    target_path == root_dir || target_path.starts_with(root_dir)
}

fn env_overrides(record: &Map<String, Value>) -> HashMap<String, String> {
    record
        .get("env")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let object = entry.as_object()?;
            let name = object.get("name")?.as_str()?;
            let value = object.get("value")?.as_str()?;
            Some((name.to_string(), value.to_string()))
        })
        .collect()
}

fn output_byte_limit(record: &Map<String, Value>) -> usize {
    record
        .get("outputByteLimit")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(DEFAULT_TERMINAL_OUTPUT_LIMIT_BYTES)
}

fn request_record<T: Serialize>(params: &T) -> Result<Map<String, Value>, ErrorSource> {
    let value = serde_json::to_value(params)?;
    let Some(record) = value.as_object() else {
        return Err("request parameters must serialize to an object".into());
    };
    Ok(record.clone())
}

fn required_string(record: &Map<String, Value>, key: &str) -> Result<String, ErrorSource> {
    record
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("Missing required field: {key}").into())
}

fn string_array(record: &Map<String, Value>, key: &str) -> Vec<String> {
    record
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect()
}

fn terminal_id_from_request<T: Serialize>(params: &T) -> Result<String, ErrorSource> {
    let record = request_record(params)?;
    let terminal_id = required_string(&record, "terminalId")?;
    let terminal_id = terminal_id.trim();
    if terminal_id.is_empty() {
        return Err("Missing required field: terminalId".into());
    }
    Ok(terminal_id.to_string())
}

fn terminal_id_from_response(response: &CreateTerminalResponse) -> Result<String, ErrorSource> {
    let record = request_record(response)?;
    required_string(&record, "terminalId")
}

fn create_terminal_response(terminal_id: String) -> CreateTerminalResponse {
    serde_json::from_value(json!({ "terminalId": terminal_id }))
        .expect("valid ACP create_terminal response")
}

fn terminal_output_response(
    output: String,
    truncated: bool,
    exit_status: Option<TerminalExitState>,
) -> TerminalOutputResponse {
    let exit_status_json = exit_status.map(|status| {
        json!({
            "exitCode": status.exit_code,
            "signal": status.signal,
        })
    });
    serde_json::from_value(json!({
        "output": output,
        "truncated": truncated,
        "exitStatus": exit_status_json,
    }))
    .expect("valid ACP terminal_output response")
}

fn wait_for_terminal_exit_response(exit_status: TerminalExitState) -> WaitForTerminalExitResponse {
    serde_json::from_value(json!({
        "exitCode": exit_status.exit_code,
        "signal": exit_status.signal,
    }))
    .expect("valid ACP wait_for_terminal_exit response")
}

fn empty_kill_terminal_response() -> KillTerminalResponse {
    serde_json::from_value(json!({})).expect("valid ACP kill_terminal response")
}

fn empty_release_terminal_response() -> ReleaseTerminalResponse {
    serde_json::from_value(json!({})).expect("valid ACP release_terminal response")
}

fn empty_release_request(terminal_id: &str) -> ReleaseTerminalRequest {
    serde_json::from_value(json!({ "sessionId": "shutdown", "terminalId": terminal_id }))
        .expect("valid ACP release_terminal request")
}

fn permission_denied_error(message: impl Into<String>) -> PermissionDeniedError {
    PermissionDeniedError::new(
        message,
        AcpxErrorOptions::default().with_defaults(
            OutputErrorCode::PermissionDenied,
            "PERMISSION_DENIED",
            OutputErrorOrigin::Runtime,
        ),
    )
}

fn unknown_terminal_error(terminal_id: &str) -> ErrorSource {
    format!("Unknown terminal: {terminal_id}").into()
}

fn next_terminal_id() -> String {
    let sequence = NEXT_TERMINAL_ID.fetch_add(1, Ordering::Relaxed);
    format!("term_{sequence:x}")
}

fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn response_record_number_or_null<T: Serialize>(value: &T, key: &str) -> String {
    request_record(value)
        .ok()
        .and_then(|record| record.get(key).and_then(Value::as_i64))
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn response_record_string_or_null<T: Serialize>(value: &T, key: &str) -> String {
    request_record(value)
        .ok()
        .and_then(|record| record.get(key).and_then(Value::as_str).map(ToOwned::to_owned))
        .unwrap_or_else(|| "null".to_string())
}

#[cfg(unix)]
fn exit_signal_name(status: &ExitStatus) -> Option<String> {
    use std::os::unix::process::ExitStatusExt;

    signal_name(status.signal()?)
}

#[cfg(not(unix))]
fn exit_signal_name(_status: &ExitStatus) -> Option<String> {
    None
}

#[cfg(unix)]
fn signal_name(signal: i32) -> Option<String> {
    let name = match signal {
        libc::SIGABRT => "SIGABRT",
        libc::SIGALRM => "SIGALRM",
        libc::SIGHUP => "SIGHUP",
        libc::SIGINT => "SIGINT",
        libc::SIGKILL => "SIGKILL",
        libc::SIGPIPE => "SIGPIPE",
        libc::SIGQUIT => "SIGQUIT",
        libc::SIGSEGV => "SIGSEGV",
        libc::SIGTERM => "SIGTERM",
        libc::SIGUSR1 => "SIGUSR1",
        libc::SIGUSR2 => "SIGUSR2",
        _ => return Some(format!("SIG{signal}")),
    };
    Some(name.to_string())
}

fn exit_status_from_failed_wait() -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(1 << 8)
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(1)
    }
}
