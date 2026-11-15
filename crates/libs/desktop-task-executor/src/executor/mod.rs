//! 不依赖桌面 App 的命令执行与输出处理。
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use desktop_task_model::models::{CLAUDE_DEFAULT_MODEL_ALIAS, TaskExecutorBackend, claude_model_alias};
use shell_command::{resolve_executable, shell_profile_env_var, std_command};

pub mod process_utils;
pub mod backend_output;
pub mod command_exec;
pub mod gateway_output;
pub mod git;
pub mod programs;

/// 公开的 TaskLogStream 枚举，描述该模块支持的一组离散状态或事件。
#[derive(Debug, Clone)]
pub enum TaskLogStream {
    Stdout(String),
    Stderr(String),
    SubTaskStarted { subtask_id: String, content: String, timestamp_ms: u64 },
    SubTaskCompleted { subtask_id: String, timestamp_ms: u64 },
    SubTaskFailed { subtask_id: String, error: String, timestamp_ms: u64 },
    ExitStatus { success: bool, code: Option<i32>, signal: Option<i32> },
}

#[cfg(not(target_arch = "wasm32"))]
fn block_on_gateway<F, T>(fut: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| handle.block_on(fut)),
        Err(_) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?;
            rt.block_on(fut)
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn block_on_gateway<F, T>(_fut: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    Err("WebAssembly 平台暂不支持同步网关调用".to_string())
}

pub mod runner;
pub mod scheduling;

use desktop_task_model::models::Task;
#[cfg(not(target_arch = "wasm32"))]
use session_text::is_default_title;
pub mod state;
pub mod worktree_admin;
mod worktree_pool;
#[cfg(test)]
mod tests;
#[cfg(not(target_arch = "wasm32"))]
fn normalize_commit_title(raw: &str) -> Option<String> {
    let mut normalized = String::new();
    let mut last_was_space = false;

    for ch in raw.chars() {
        if ch.is_control() {
            if matches!(ch, '\n' | '\r' | '\t') && !last_was_space {
                normalized.push(' ');
                last_was_space = true;
            }
            continue;
        }

        if ch.is_whitespace() {
            if !last_was_space {
                normalized.push(' ');
                last_was_space = true;
            }
            continue;
        }

        normalized.push(ch);
        last_was_space = false;
    }

    let normalized = normalized.trim();
    if normalized.is_empty() {
        return None;
    }

    let limited = normalized.chars().take(72).collect::<String>();
    Some(limited)
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_non_default_commit_title(raw: &str) -> Option<String> {
    let normalized = normalize_commit_title(raw)?;
    if normalized == "新会话" || is_default_title(&normalized) {
        return None;
    }
    Some(normalized)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 task_session_id 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn task_session_id(task_id: &str) -> String {
    format!("task-board-{}", task_id)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn load_task_session_title(gateway_client: fn() -> Result<vw_gateway_client::GatewayClient, String>, task: &Task) -> Option<String> {
    let session_id = task_session_id(&task.id);
    let client = gateway_client().ok()?;
    let preview = block_on_gateway(gateway_session::GatewaySessionClient::new(client.transport().clone()).session_preview_meta_get(&session_id, None)).ok().flatten()?;
    normalize_non_default_commit_title(&preview.title)
        .or_else(|| preview.last_content.as_deref().and_then(normalize_commit_title))
}

#[cfg(target_arch = "wasm32")]
/// 执行 load_task_session_title 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn load_task_session_title(_gateway_client: fn() -> Result<vw_gateway_client::GatewayClient, String>, _task: &Task) -> Option<String> {
    None
}

/// 执行 build_task_commit_message 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn build_task_commit_message(gateway_client: fn() -> Result<vw_gateway_client::GatewayClient, String>, task: &Task, fallback: &str) -> String {
    match load_task_session_title(gateway_client, task) {
        Some(title) => format!("task({}): {}", task.id, title),
        None => fallback.to_string(),
    }
}
