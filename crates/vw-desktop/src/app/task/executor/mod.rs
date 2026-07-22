//! 实现任务执行器的命令调度、进程输出和辅助处理。
//!
//! 注释说明当前文件的职责边界，帮助调用方理解数据流与错误传播，
//! 不改变任何运行时行为。

use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use super::models::{
    CLAUDE_DEFAULT_MODEL_ALIAS, Task, TaskExecutorBackend, TaskStatus, claude_model_alias,
};
use super::store;
#[cfg(not(target_arch = "wasm32"))]
use vw_shared::session::session_utils::is_default_title;
#[cfg(not(target_arch = "wasm32"))]
use vw_shared::session::ui_types::ChatRole;
use vw_shared::shell::{resolve_executable, shell_profile_env_var, std_command};

mod backend_output;
mod command_exec;
mod git;
mod process_utils;
mod programs;
mod runner;
mod scheduling;
mod state;
mod worktree_admin;
mod worktree_pool;

#[cfg(test)]
mod tests;

/// 对外暴露当前模块需要复用的能力。
pub use command_exec::{execute_task_command, execute_task_command_with_streaming};
/// 对外暴露当前模块需要复用的能力。
pub use git::build_review_diff_context;
/// 对外暴露当前模块需要复用的能力。
pub use programs::{ExecutorCommand, build_executor_command};
/// 对外暴露当前模块需要复用的能力。
pub use runner::{
    TaskPlanGenerationOutcome, TaskPlanSubTask, execute_gateway_prompt_with_streaming,
    execute_task_async, execute_task_merge_async, execute_task_plan_async,
    execute_task_retry_async, execute_task_review_async, write_task_plan_files,
};
/// 对外暴露当前模块需要复用的能力。
pub use scheduling::{
    ExecutorEvent, count_running_tasks, get_next_tasks_for_execution, get_pool_and_pending_count,
    get_total_task_count, simulate_task_execution_step,
};
/// 对外暴露当前模块需要复用的能力。
pub use state::{
    TaskExecutorState, TaskLogStream, WorktreePoolSnapshot, WorktreeSlotSnapshot, WorktreeState,
};
/// 对外暴露当前模块需要复用的能力。
pub use worktree_admin::{
    assign_task_execution_worktree, commit_merge_all_worktrees, commit_merge_all_worktrees_async,
    commit_merge_all_worktrees_async_with_logs, commit_merge_all_worktrees_with_logs,
    delete_all_managed_worktrees, delete_all_managed_worktrees_async,
    delete_all_managed_worktrees_async_with_logs, delete_all_managed_worktrees_with_logs,
    reset_all_managed_worktrees, reset_all_managed_worktrees_async,
    reset_all_managed_worktrees_async_with_logs, reset_all_managed_worktrees_with_logs,
};

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

#[cfg(not(target_arch = "wasm32"))]
fn persist_task_session_title(session_id: &str, title: &str) {
    if let Ok(client) = crate::app::config::gateway_client() {
        let _ = block_on_gateway(client.session_update::<()>(
            session_id,
            None,
            &vw_gateway_client::GatewaySessionPatchBody {
                title: Some(title.to_string()),
                time: None,
            },
        ));
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn generate_task_session_title_from_content(session_id: &str, content: &str) -> Option<String> {
    let client = crate::app::config::gateway_client().ok()?;
    let generated = block_on_gateway(client.session_title_generate(
        session_id,
        &vw_gateway_client::GatewaySessionTitleGenerateBody {
            content: content.to_string(),
            preferred_model: None,
            acp_agent: None,
        },
    ))
    .ok()?
    .title;
    let title = normalize_non_default_commit_title(&generated)
        .or_else(|| normalize_commit_title(content))?;
    persist_task_session_title(session_id, &title);
    Some(title)
}

#[cfg(not(target_arch = "wasm32"))]
/// 执行 load_task_session_title 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn load_task_session_title(task: &Task) -> Option<String> {
    let session_id = task_session_id(&task.id);

    if let Some(title) = crate::app::session_gateway::gateway_session_preview_meta(&session_id)
        .and_then(|meta| normalize_non_default_commit_title(&meta.title))
    {
        return Some(title);
    }

    let session = crate::app::session_gateway::gateway_load_session_any(&session_id);
    if let Some(title) =
        session.as_ref().and_then(|session| normalize_non_default_commit_title(&session.title))
    {
        return Some(title);
    }

    let first_user_content = session.as_ref().and_then(|session| {
        session.messages.iter().find_map(|message| {
            (message.role == ChatRole::User)
                .then_some(message.content.trim())
                .filter(|content| !content.is_empty())
                .map(str::to_string)
        })
    });

    let fallback_content = first_user_content
        .as_deref()
        .or_else(|| Some(task.prompt.trim()).filter(|content| !content.is_empty()))?;

    generate_task_session_title_from_content(&session_id, fallback_content)
        .or_else(|| normalize_commit_title(fallback_content))
}

#[cfg(target_arch = "wasm32")]
/// 执行 load_task_session_title 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn load_task_session_title(_task: &Task) -> Option<String> {
    None
}

/// 执行 build_task_commit_message 对应的领域操作。
///
/// 参数由调用方提供，函数在当前模块的状态边界内完成处理。
/// 返回值表达处理结果；失败时保留错误信息给上层界面或调度逻辑。
pub(super) fn build_task_commit_message(task: &Task, fallback: &str) -> String {
    match load_task_session_title(task) {
        Some(title) => format!("task({}): {}", task.id, title),
        None => fallback.to_string(),
    }
}
