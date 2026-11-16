//! 会话指令文件发现与加载。
//!
//! 本模块负责发现 AGENTS/CLAUDE/CONTEXT 等指令文件，并控制它们何时注入会话上下文。
//! 它会跟踪每条消息已经声明过的指令文件，避免同一轮对话重复加载相同内容。

use agent_config::runtime as config;
use runtime_flags as flag;
use agent_project::instance;
use agent_security::SecurityPolicy;
use crate::message;
use agent_util::filesystem;
use agent_log as log;
pub use session_instructions::{Resolved, InstructionSource, State, agent_workspace_root, agent_workspace_identity, find};
use session_instructions::{normalize_str, resolved_file};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::LazyLock;

static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("instruction".to_string()));
        m
    }))
});

/// 获取项目实例级别的指令状态工厂。
///
/// 返回的闭包按当前项目实例隔离状态，避免不同工作区之间共享“已声明”记录。
fn instance_state()
-> impl Fn() -> agent_project::BoxFuture<Arc<State>> + Send + Sync + 'static {
    instance::state(
        "instruction",
        || async { State::default() },
        None::<fn(Arc<State>) -> agent_project::BoxFuture<()>>,
    )
}

/// 向上解析相对指令文件名。
///
/// 当项目配置启用时，从项目目录向工作树边界查找；禁用项目配置时，只在
/// `VIBEWINDOW_CONFIG_DIR` 内查找，避免越过显式配置沙箱。
pub fn resolve_relative(instruction: &str) -> Vec<PathBuf> {
    if !flag::vibewindow_disable_project_config() {
        let start = PathBuf::from(instance::directory());
        let stop = {
            let wt = instance::worktree();
            if wt.is_empty() { None } else { Some(PathBuf::from(wt)) }
        };
        return filesystem::glob_up(instruction, start, stop);
    }

    let Some(dir) = flag::vibewindow_config_dir() else {
        LOGGER.warn(
            format!(
                "Skipping relative instruction \"{}\" - no VIBEWINDOW_CONFIG_DIR set while project config is disabled",
                instruction
            ),
            None,
        );
        return Vec::new();
    };
    let start = PathBuf::from(dir.clone());
    filesystem::glob_up(instruction, start.clone(), Some(start))
}

/// Query the project-local claim state for host integration tests.
pub async fn is_claimed(message_id: &str, filepath: &str) -> bool {
    instance_state()().await.is_claimed(message_id, filepath).await
}

pub async fn claim(message_id: &str, filepath: &str) {
    instance_state()().await.claim(message_id, filepath).await;
}

/// Clear this message's claims and instruction cache.
pub async fn clear(message_id: &str) {
    instance_state()().await.clear(&filesystem::normalize_path(instance::directory()), message_id).await;
}

/// Reload instructions after compaction within the same project state.
pub async fn reload_after_compaction(message_id: &str) {
    instance_state()().await.reload_after_compaction(&filesystem::normalize_path(instance::directory()), message_id).await;
}

/// 查找当前系统级指令文件路径集合。
///
/// 返回所有 AGENTS 来源的规范化路径集合。
pub async fn system_paths() -> HashSet<String> {
    let _cfg = config::get().await;
    crate::instruction_discovery::system_sources()
        .into_iter()
        .filter(|source| source.kind != crate::instruction_discovery::SourceKind::Rule)
        .map(|source| normalize_str(source.filepath))
        .collect()
}

/// 读取系统级指令内容及其来源路径。
///
/// 返回每个非空指令文件的路径与可注入文本。读取失败按空内容处理，使缺失或临时
/// 不可读的指令文件不会中断会话创建。
pub async fn system_resolved() -> Vec<Resolved> {
    let config = config::get().await;
    let workspace = filesystem::normalize_path(instance::directory());
    let policy = instruction_policy(&config, &workspace);
    let mut out = Vec::new();
    for source in crate::instruction_discovery::system_sources() {
        if source.kind == crate::instruction_discovery::SourceKind::Rule {
            continue;
        }
        if let Some(resolved) = resolved_file(&source.filepath, &policy, Vec::new()) {
            out.push(resolved);
        }
    }

    out
}

/// 从请求显式选择的工作区加载首步 AGENTS 上下文。
///
/// 该入口不依赖项目 task-local，供跨线程的桌面、计划执行和子代理入口使用。
pub async fn workspace_resolved(workspace: &Path) -> Vec<Resolved> {
    let config = config::get().await;
    let workspace = filesystem::normalize_path(workspace);
    let policy = instruction_policy(&config, &workspace);

    crate::instruction_discovery::workspace_sources(&workspace)
        .into_iter()
        .filter(|source| source.kind != crate::instruction_discovery::SourceKind::Rule)
        .filter_map(|source| resolved_file(&source.filepath, &policy, Vec::new()))
        .collect()
}

/// 读取系统级指令内容。
///
/// 返回每个指令文件的可注入文本，空文件会被跳过。
pub async fn system() -> Vec<String> {
    system_resolved().await.into_iter().map(|resolved| resolved.content).collect()
}

/// 读取当前请求所选项目根目录下的 `AGENTS.md`。
///
/// 该来源独立于系统/全局指令发现：调用方可将项目协议追加到系统提示词，同时只把
/// 此路径暴露给界面。AGENTS 正文不会触发技能名称或路径猜测，技能仅由独立的
/// Skill Tool 链路按需加载。项目配置被禁用、根目录为空、文件缺失或内容为空时返回
/// `None`。
pub async fn project_agents(root: Option<&str>) -> Option<Resolved> {
    if flag::vibewindow_disable_project_config() || root.is_none_or(|root| root.trim().is_empty()) {
        return None;
    }
    let config = config::get().await;
    project_agents_with_config(root, &config).await
}

pub async fn project_agents_with_config(
    root: Option<&str>,
    config: &config::Config,
) -> Option<Resolved> {
    if flag::vibewindow_disable_project_config() {
        return None;
    }
    let root = root.map(str::trim).filter(|root| !root.is_empty())?;
    let root_path = PathBuf::from(root);
    let filepath = root_path.join("AGENTS.md");
    if !filesystem::exists(&filepath) {
        return None;
    }
    let policy = instruction_policy(config, &root_path);
    resolved_file(&filepath, &policy, Vec::new())
}

/// 从历史消息中提取已经加载过的指令路径。
///
/// `messages` 是会话消息列表。返回集合只包含未被 compact 的 read/file_read 工具
/// 结果中标记为 `loaded` 的路径。
pub fn loaded(messages: &[message::WithParts]) -> HashSet<String> {
    let mut paths = HashSet::<String>::new();
    for msg in messages {
        for part in &msg.parts {
            let message::Part::Tool(tp) = part else { continue };
            if tp.tool != "read" && tp.tool != "file_read" {
                continue;
            }
            let message::ToolState::Completed(st) = &tp.state else { continue };
            if st.time.compacted.is_some() {
                continue;
            }
            let Some(loaded) = st.metadata.get("loaded").and_then(Value::as_array) else {
                continue;
            };
            for p in loaded {
                if let Some(s) = p.as_str() {
                    paths.insert(s.to_string());
                }
            }
        }
    }
    paths
}

fn instruction_policy(config: &config::Config, workspace: &Path) -> SecurityPolicy {
    SecurityPolicy::from_configs(&config.autonomy, &config.security, workspace)
}

/// 解析目标文件上级目录中的附加指令。
///
/// `messages` 用于避免重复加载已读指令，`filepath` 是当前目标文件，`message_id`
/// 用于本轮声明去重。返回从目标文件目录向项目根逐级发现的指令内容，不包含系统
/// 指令、目标文件自身或已加载/已声明的文件。
pub async fn resolve(
    messages: &[message::WithParts],
    filepath: &str,
    message_id: &str,
) -> Vec<Resolved> {
    let system = system_paths().await;
    let already = loaded(messages);
    let root = filesystem::normalize_path(instance::directory());
    let config = config::get().await;
    let policy = instruction_policy(&config, &root);
    let state = instance_state()().await;
    session_instructions::resolve(&state, &system, &already, filepath, message_id, root, &policy).await
}

#[cfg(test)]
#[path = "instruction_tests.rs"]
mod instruction_tests;
