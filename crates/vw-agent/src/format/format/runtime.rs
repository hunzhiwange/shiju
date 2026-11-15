//! 文件保存后的自动格式化运行时。
//!
//! 该模块订阅文件编辑事件，按扩展名选择已启用的格式化器，并在后台 Tokio
//! runtime 中执行格式化命令。命令失败只记录日志，不阻断文件编辑流程。

use super::detect::is_enabled;
use super::state::{FormatterStatus, LOGGER, instance_state};
use crate::app::agent::bus;
use agent_files as file;
use crate::app::agent::project::instance;
use crate::app::agent::shell::tokio_command;
use serde_json::{Map, Value};
use std::path::Path;
use std::sync::LazyLock;
use std::sync::OnceLock;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn format_runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(8)
            .enable_all()
            .build()
            .unwrap_or_else(|_| tokio::runtime::Runtime::new().expect("create tokio runtime"))
    })
}

#[cfg(target_arch = "wasm32")]
/// 在 wasm 构建中初始化格式化运行时。
///
/// # 说明
///
/// wasm 环境不启动本地进程，因此该函数为空实现。
pub fn init() {}

#[cfg(not(target_arch = "wasm32"))]
/// 初始化本地格式化运行时。
///
/// # 说明
///
/// 该函数只会完成一次订阅；后续文件编辑事件会触发后台格式化任务。
pub fn init() {
    static INIT: LazyLock<()> = LazyLock::new(|| {
        LOGGER.info("init", None);
        bus::subscribe(file::event::EDITED, move |evt| {
            let Some(props) = evt.get("properties") else {
                return;
            };
            let Some(file) = props.get("file").and_then(|v| v.as_str()) else {
                return;
            };
            let file = file.to_string();

            let rt = format_runtime();
            rt.spawn(async move {
                let _ = format_file(&file).await;
            });
        });
    });
    LazyLock::force(&INIT);
}

/// 返回当前实例中所有内置格式化器的状态。
///
/// # 返回值
///
/// 每个条目包含格式化器名称、适用扩展名以及当前工作区是否启用。
pub async fn status() -> Vec<FormatterStatus> {
    let state = instance_state()().await;
    let mut out = Vec::new();

    for formatter in state.formatters.values() {
        let enabled = is_enabled(&state, formatter).await;
        out.push(FormatterStatus {
            name: formatter.name.clone(),
            extensions: formatter.extensions.clone(),
            enabled,
        });
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) async fn format_file(filepath: &str) -> Result<(), ()> {
    let ext = Path::new(filepath)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{}", s))
        .unwrap_or_default();
    if ext.is_empty() {
        return Ok(());
    }

    let state = instance_state()().await;
    let mut candidates = Vec::new();
    for formatter in state.formatters.values() {
        if !formatter.extensions.iter().any(|entry| entry == &ext) {
            continue;
        }
        if !is_enabled(&state, formatter).await {
            continue;
        }
        candidates.push(formatter.clone());
    }
    if candidates.is_empty() {
        return Ok(());
    }

    let cwd = instance::directory();
    for formatter in candidates {
        // 命令模板只替换文件路径，不经 shell 拼接，避免格式化参数被解释为脚本。
        let command = formatter
            .command
            .iter()
            .map(|item| item.replace("$FILE", filepath))
            .collect::<Vec<_>>();
        if command.is_empty() {
            continue;
        }

        LOGGER.info(
            "running",
            Some({
                let mut m = Map::new();
                m.insert("name".to_string(), Value::String(formatter.name.clone()));
                m.insert("file".to_string(), Value::String(filepath.to_string()));
                m
            }),
        );

        let mut cmd = tokio_command(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }
        if !cwd.trim().is_empty() {
            cmd.current_dir(&cwd);
        }
        for (key, value) in formatter.environment {
            cmd.env(key, value);
        }
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        let status = cmd.status().await;
        if status.as_ref().is_ok_and(|s| s.success()) {
            continue;
        }

        LOGGER.error(
            "failed",
            Some({
                let mut m = Map::new();
                m.insert("name".to_string(), Value::String(formatter.name.clone()));
                m.insert("file".to_string(), Value::String(filepath.to_string()));
                m.insert(
                    "command".to_string(),
                    Value::String(command.into_iter().collect::<Vec<_>>().join(" ")),
                );
                m
            }),
        );
    }

    Ok(())
}
