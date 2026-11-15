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
