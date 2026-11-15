//! 队列所有者子进程的参数构造与启动逻辑。

use std::collections::HashMap;
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use agent_client_protocol::McpServer;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use acp_types::{AuthPolicy, NonInteractivePermissionPolicy, PermissionMode};

pub const QUEUE_OWNER_ARGS_ENV: &str = "VWACP_QUEUE_OWNER_ARGS";
pub const QUEUE_OWNER_PAYLOAD_ENV: &str = "VWACP_QUEUE_OWNER_PAYLOAD";
pub const QUEUE_OWNER_PROCESS_MARKER: &str = "__queue-owner";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueOwnerRuntimeOptions {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<McpServer>>,
    pub permission_mode: PermissionMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_credentials: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_policy: Option<AuthPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_sdk_console_errors: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verbose: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_queue_depth: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_retries: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueOwnerRuntimeSendOptions {
    pub session_id: String,
    pub mcp_servers: Option<Vec<McpServer>>,
    pub permission_mode: PermissionMode,
    pub non_interactive_permissions: Option<NonInteractivePermissionPolicy>,
    pub auth_credentials: Option<HashMap<String, String>>,
    pub auth_policy: Option<AuthPolicy>,
    pub suppress_sdk_console_errors: Option<bool>,
    pub verbose: Option<bool>,
    pub ttl_ms: Option<u64>,
    pub max_queue_depth: Option<usize>,
    pub prompt_retries: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueOwnerSpawnCommand {
    pub executable_path: PathBuf,
    pub args: Vec<String>,
}

#[derive(Debug, Error)]
pub enum QueueOwnerProcessError {
    #[error("vwacp self-spawn failed: invalid {QUEUE_OWNER_ARGS_ENV}")]
    InvalidArgsOverride,
    #[error("vwacp self-spawn failed: missing current executable path")]
    MissingCurrentExecutable,
    #[error("vwacp self-spawn failed: missing executable path")]
    MissingExecutablePath,
    #[error("vwacp self-spawn failed: unable to resolve current executable")]
    ResolveCurrentExecutable(#[source] io::Error),
    #[error("vwacp self-spawn failed: unable to canonicalize executable path")]
    CanonicalizeExecutable(#[source] io::Error),
    #[error("vwacp self-spawn failed: unable to serialize {QUEUE_OWNER_PAYLOAD_ENV}")]
    SerializePayload(#[source] serde_json::Error),
    #[error("vwacp self-spawn failed: unable to spawn queue owner process")]
    Spawn(#[source] io::Error),
}

pub fn sanitize_queue_owner_exec_argv(exec_argv: &[String]) -> Vec<String> {
    let mut sanitized = Vec::new();
    let mut index = 0;

    while index < exec_argv.len() {
        let value = &exec_argv[index];

        if value == "--experimental-test-coverage" || value == "--test" {
            index += 1;
            continue;
        }

        if matches!(
            value.as_str(),
            "--test-name-pattern" | "--test-reporter" | "--test-reporter-destination"
        ) {
            index += 2;
            continue;
        }

        if value.starts_with("--test-") {
            index += 1;
            continue;
        }

        if matches!(
            value.as_str(),
            "--inspect"
                | "--inspect-brk"
                | "--inspect-port"
                | "--inspect-publish-uid"
                | "--debug-port"
        ) {
            index += 2;
            continue;
        }

        if value.starts_with("--inspect=")
            || value.starts_with("--inspect-brk=")
            || value.starts_with("--inspect-port=")
            || value.starts_with("--inspect-publish-uid=")
            || value.starts_with("--debug-port=")
        {
            index += 1;
            continue;
        }

        sanitized.push(value.clone());
        index += 1;
    }

    sanitized
}

pub fn build_queue_owner_arg_override(
    executable_path: &Path,
    exec_argv: &[String],
) -> Option<String> {
    let sanitized = sanitize_queue_owner_exec_argv(exec_argv);
    if sanitized.is_empty() {
        return None;
    }

    let mut override_args = Vec::with_capacity(sanitized.len() + 2);
    override_args.push(executable_path.to_string_lossy().into_owned());
    override_args.extend(sanitized);
    override_args.push(QUEUE_OWNER_PROCESS_MARKER.to_string());
    Some(serde_json::json!(override_args).to_string())
}

pub fn resolve_queue_owner_spawn_args(
    current_executable: Option<&Path>,
) -> Result<Vec<String>, QueueOwnerProcessError> {
    let args_override = env::var(QUEUE_OWNER_ARGS_ENV).ok();
    resolve_queue_owner_spawn_args_with_override(current_executable, args_override.as_deref())
}

pub fn resolve_queue_owner_spawn_command(
    current_executable: Option<&Path>,
) -> Result<QueueOwnerSpawnCommand, QueueOwnerProcessError> {
    let args = resolve_queue_owner_spawn_args(current_executable)?;
    let (executable_path, args) =
        args.split_first().ok_or(QueueOwnerProcessError::MissingExecutablePath)?;

    Ok(QueueOwnerSpawnCommand {
        executable_path: PathBuf::from(executable_path),
        args: args.to_vec(),
    })
}

pub fn queue_owner_runtime_options_from_send(
    options: &QueueOwnerRuntimeSendOptions,
) -> QueueOwnerRuntimeOptions {
    QueueOwnerRuntimeOptions {
        session_id: options.session_id.clone(),
        mcp_servers: options.mcp_servers.clone(),
        permission_mode: options.permission_mode,
        non_interactive_permissions: options.non_interactive_permissions,
        auth_credentials: options.auth_credentials.clone(),
        auth_policy: options.auth_policy,
        suppress_sdk_console_errors: options.suppress_sdk_console_errors,
        verbose: options.verbose,
        ttl_ms: options.ttl_ms,
        max_queue_depth: options.max_queue_depth,
        prompt_retries: options.prompt_retries,
    }
}

pub fn spawn_queue_owner_process(
    options: &QueueOwnerRuntimeOptions,
    current_executable: Option<&Path>,
) -> Result<(), QueueOwnerProcessError> {
    let payload =
        serde_json::to_string(options).map_err(QueueOwnerProcessError::SerializePayload)?;
    let spawn_command = resolve_queue_owner_spawn_command(current_executable)?;
    let mut command = Command::new(&spawn_command.executable_path);
    command
        .args(&spawn_command.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .env(QUEUE_OWNER_PAYLOAD_ENV, payload);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;

        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    command.spawn().map_err(QueueOwnerProcessError::Spawn)?;
    Ok(())
}

pub(crate) fn resolve_queue_owner_spawn_args_with_override(
    current_executable: Option<&Path>,
    args_override: Option<&str>,
) -> Result<Vec<String>, QueueOwnerProcessError> {
    if let Some(args_override) = args_override {
        let parsed = serde_json::from_str::<Vec<String>>(args_override)
            .map_err(|_| QueueOwnerProcessError::InvalidArgsOverride)?;
        if parsed.is_empty() || parsed.iter().any(|value| value.trim().is_empty()) {
            return Err(QueueOwnerProcessError::InvalidArgsOverride);
        }
        return Ok(parsed);
    }

    let executable_path = match current_executable {
        Some(path) if !path.as_os_str().is_empty() => path.to_path_buf(),
        Some(_) => return Err(QueueOwnerProcessError::MissingCurrentExecutable),
        None => env::current_exe().map_err(QueueOwnerProcessError::ResolveCurrentExecutable)?,
    };
    let executable_path =
        executable_path.canonicalize().map_err(QueueOwnerProcessError::CanonicalizeExecutable)?;

    Ok(vec![executable_path.to_string_lossy().into_owned(), QUEUE_OWNER_PROCESS_MARKER.to_string()])
}
