//! 会话指令文件发现与加载。
//!
//! 本模块负责发现 AGENTS/CLAUDE/CONTEXT 等指令文件，并控制它们何时注入会话上下文。
//! 它会跟踪每条消息已经声明过的指令文件，避免同一轮对话重复加载相同内容。

use crate::app::agent::config;
use crate::app::agent::flag;
use crate::app::agent::project::instance;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::session::message;
use crate::app::agent::util::filesystem;
use crate::app::agent::util::log;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::Mutex;

#[path = "instruction_cache.rs"]
mod instruction_cache;
#[path = "instruction_include.rs"]
mod instruction_include;
#[path = "instruction_rules.rs"]
mod instruction_rules;

#[cfg(test)]
#[path = "instruction_cache_tests.rs"]
mod instruction_cache_tests;

#[cfg(test)]
#[path = "instruction_include_tests.rs"]
mod instruction_include_tests;
#[cfg(test)]
#[path = "instruction_rules_tests.rs"]
mod instruction_rules_tests;

#[cfg(test)]
#[path = "claude_protocol_parity_tests.rs"]
mod claude_protocol_parity_tests;

static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("instruction".to_string()));
        m
    }))
});

const WORKSPACE_IDENTITY_FILES: [&str; 7] =
    ["AGENTS.md", "SOUL.md", "TOOLS.md", "IDENTITY.md", "USER.md", "HEARTBEAT.md", "MEMORY.md"];

/// 单个项目实例内的指令加载状态。
#[derive(Debug, Default)]
struct State {
    claims: Mutex<HashMap<String, HashSet<String>>>,
    cache: Mutex<instruction_cache::InstructionCache>,
}

/// 获取项目实例级别的指令状态工厂。
///
/// 返回的闭包按当前项目实例隔离状态，避免不同工作区之间共享“已声明”记录。
fn instance_state()
-> impl Fn() -> crate::app::agent::project::BoxFuture<Arc<State>> + Send + Sync + 'static {
    instance::state(
        "instruction",
        || async { State::default() },
        None::<fn(Arc<State>) -> crate::app::agent::project::BoxFuture<()>>,
    )
}

/// 将路径规范化为可比较的字符串。
fn normalize_str(p: impl AsRef<Path>) -> String {
    filesystem::normalize_path(p).to_string_lossy().to_string()
}

/// 向上解析相对指令文件名。
///
/// 当项目配置启用时，从项目目录向工作树边界查找；禁用项目配置时，只在
/// `VIBEWINDOW_CONFIG_DIR` 内查找，避免越过显式配置沙箱。
fn resolve_relative(instruction: &str) -> Vec<PathBuf> {
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

/// 判断某条消息是否已经声明过指定指令文件。
async fn is_claimed(message_id: &str, filepath: &str) -> bool {
    let state = instance_state()().await;
    let claims = state.claims.lock().await;
    claims.get(message_id).is_some_and(|s| s.contains(filepath))
}

/// 标记某条消息已经声明指定指令文件。
async fn claim(message_id: &str, filepath: &str) {
    let state = instance_state()().await;
    let mut claims = state.claims.lock().await;
    claims.entry(message_id.to_string()).or_default().insert(filepath.to_string());
}

/// 清理指定消息的指令声明记录。
///
/// `message_id` 是会话消息 ID。清理后，同一消息关联的文件可在后续流程重新声明。
pub async fn clear(message_id: &str) {
    let state = instance_state()().await;
    let mut claims = state.claims.lock().await;
    claims.remove(message_id);
    drop(claims);
    state
        .cache
        .lock()
        .await
        .clear_session(&filesystem::normalize_path(instance::directory()), message_id);
}

/// 让压缩后的会话在下次访问时重新读取所有已解析的指令。
pub async fn reload_after_compaction(message_id: &str) {
    let state = instance_state()().await;
    state.claims.lock().await.remove(message_id);
    state
        .cache
        .lock()
        .await
        .invalidate_for_compaction(&filesystem::normalize_path(instance::directory()), message_id);
}

/// 查找当前系统级指令文件路径集合。
///
/// 返回所有 AGENTS 来源的规范化路径集合。
pub async fn system_paths() -> HashSet<String> {
    let _cfg = config::get().await;
    crate::session::instruction_discovery::system_sources()
        .into_iter()
        .filter(|source| source.kind != crate::session::instruction_discovery::SourceKind::Rule)
        .map(|source| normalize_str(source.filepath))
        .collect()
}

#[cfg(target_arch = "wasm32")]
/// wasm 架构下的 URL 指令占位加载器。
async fn fetch_url(_url: &str) -> String {
    String::new()
}

#[cfg(not(target_arch = "wasm32"))]
/// 在非 wasm 环境中读取远程 URL 指令内容。
///
/// 网络请求放入阻塞线程并设置短超时；失败时返回空字符串，避免指令加载阻塞主会话。
async fn fetch_url(url: &str) -> String {
    let url = url.to_string();
    tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .ok()?;
        let res = client.get(url).send().ok()?;
        if !res.status().is_success() {
            return None;
        }
        res.text().ok()
    })
    .await
    .ok()
    .flatten()
    .unwrap_or_default()
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
    for source in crate::session::instruction_discovery::system_sources() {
        if source.kind == crate::session::instruction_discovery::SourceKind::Rule {
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

    crate::session::instruction_discovery::workspace_sources(&workspace)
        .into_iter()
        .filter(|source| source.kind != crate::session::instruction_discovery::SourceKind::Rule)
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

async fn project_agents_with_config(
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

/// 返回指定委托代理的身份工作区根目录。
pub fn agent_workspace_root(agent_key: Option<&str>) -> Option<PathBuf> {
    let agent_key = agent_key.unwrap_or("main").trim();
    let suffix = if agent_key.is_empty() || agent_key == "main" {
        String::new()
    } else {
        format!("-{agent_key}")
    };

    if let Ok(workspace) = std::env::var("VIBEWINDOW_WORKSPACE") {
        let workspace = workspace.trim();
        if !workspace.is_empty() {
            return Some(PathBuf::from(format!("{workspace}{suffix}")));
        }
    }

    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from).map(
        |home| vw_config_types::paths::home_config_dir(home).join(format!("workspace{suffix}")),
    )
}

/// 读取指定委托代理工作区中已启用、实际存在且非空的身份文件。
pub fn agent_workspace_identity(
    agent_key: Option<&str>,
    enabled_identity_files: &[String],
) -> Vec<Resolved> {
    let Some(root) = agent_workspace_root(agent_key) else {
        return Vec::new();
    };

    WORKSPACE_IDENTITY_FILES
        .into_iter()
        .filter(|filename| enabled_identity_files.iter().any(|enabled| enabled == filename))
        .filter_map(|filename| {
            let filepath = root.join(filename);
            if !filesystem::exists(&filepath) {
                return None;
            }
            let filepath = normalize_str(filepath);
            let content = std::fs::read_to_string(&filepath).unwrap_or_default();
            if content.trim().is_empty() {
                return None;
            }
            Some(Resolved {
                filepath: filepath.clone(),
                content: format!("Instructions from: {}\n{}", filepath, content),
                sources: vec![InstructionSource { filepath, parent: None, conditions: Vec::new() }],
                diagnostics: Vec::new(),
            })
        })
        .collect()
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

/// 在指定目录查找优先级最高的 AGENTS 指令文件。
///
/// 返回规范化路径；如果目录下没有支持的文件，返回 `None`。
pub async fn find(dir: &str) -> Option<String> {
    crate::session::instruction_discovery::directory_sources(Path::new(dir))
        .into_iter()
        .next()
        .map(|source| normalize_str(source.filepath))
}

/// 已解析出的附加指令文件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// 指令文件的规范化路径。
    pub filepath: String,
    /// 带来源头的指令内容。
    pub content: String,
    /// 展开后参与该指令的文件来源及父子关系。
    pub sources: Vec<InstructionSource>,
    /// 未授权、缺失或循环包含等明确诊断。
    pub diagnostics: Vec<String>,
}

/// 展开后单个指令文本的来源和激活条件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionSource {
    /// 规范化后的来源文件路径。
    pub filepath: String,
    /// 包含该文件的父文件；根来源为 `None`。
    pub parent: Option<String>,
    /// 激活该来源所需的路径条件。
    pub conditions: Vec<String>,
}

fn instruction_policy(config: &config::Config, workspace: &Path) -> SecurityPolicy {
    SecurityPolicy::from_configs(&config.autonomy, &config.security, workspace)
}

fn resolved_file(
    filepath: &Path,
    policy: &SecurityPolicy,
    conditions: Vec<String>,
) -> Option<Resolved> {
    let expanded = instruction_include::expand(filepath, policy);
    if expanded.content.trim().is_empty() {
        return None;
    }
    let filepath = normalize_str(filepath);
    Some(Resolved {
        filepath: filepath.clone(),
        content: format!("Instructions from: {}\n{}", filepath, expanded.content),
        sources: expanded
            .sources
            .into_iter()
            .map(|source| InstructionSource {
                filepath: source.filepath,
                parent: source.parent,
                conditions: conditions.clone(),
            })
            .collect(),
        diagnostics: expanded.diagnostics,
    })
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
    let mut results: Vec<Resolved> = Vec::new();

    let target = normalize_str(filepath);
    let root = filesystem::normalize_path(instance::directory());
    let config = config::get().await;
    let policy = instruction_policy(&config, &root);
    let mut current = PathBuf::from(&target)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(&target));

    while current.starts_with(&root) {
        for source in crate::session::instruction_discovery::directory_sources(&current) {
            let found = normalize_str(&source.filepath);
            if found != target
                && !system.contains(&found)
                && !already.contains(&found)
                && !is_claimed(message_id, &found).await
            {
                // claim 必须在读取前完成，防止并发解析同一消息时重复注入相同指令。
                claim(message_id, &found).await;
                if source.kind == crate::session::instruction_discovery::SourceKind::Rule {
                    let Some(rule) = instruction_rules::load_if_matches(
                        &source.filepath,
                        &root,
                        Path::new(&target),
                    ) else {
                        continue;
                    };
                    results.push(Resolved {
                        filepath: found.clone(),
                        content: format!("Instructions from: {}\n{}", found, rule.content),
                        sources: vec![InstructionSource {
                            filepath: found.clone(),
                            parent: None,
                            conditions: rule.conditions,
                        }],
                        diagnostics: Vec::new(),
                    });
                } else {
                    let state = instance_state()().await;
                    let mut cache = state.cache.lock().await;
                    let (resolved, _) = cache.load(&root, message_id, &source.filepath, || {
                        resolved_file(&source.filepath, &policy, Vec::new())
                    });
                    if let Some(resolved) = resolved {
                        results.push(resolved);
                    }
                }
            }
        }

        if current == root {
            break;
        }
        current = current.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| root.clone());
    }

    results
}
#[cfg(test)]
#[path = "instruction_tests.rs"]
mod instruction_tests;
