//! 会话指令文件发现与加载。
//!
//! 本模块负责发现 AGENTS/CLAUDE/CONTEXT 等指令文件，并控制它们何时注入会话上下文。
//! 它会跟踪每条消息已经声明过的指令文件，避免同一轮对话重复加载相同内容。

use crate::app::agent::config;
use crate::app::agent::flag;
use crate::app::agent::global;
use crate::app::agent::project::instance;
use crate::app::agent::session::message;
use crate::app::agent::skills::{
    SkillLoadMode, load_skills_from_directory, skills_to_prompt_with_mode,
};
use crate::app::agent::util::filesystem;
use crate::app::agent::util::log;
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::Mutex;

static LOGGER: LazyLock<log::Logger> = LazyLock::new(|| {
    log::create(Some({
        let mut m = Map::new();
        m.insert("service".to_string(), Value::String("instruction".to_string()));
        m
    }))
});

const FILES: [&str; 3] = ["AGENTS.md", "CLAUDE.md", "CONTEXT.md"];
const WORKSPACE_IDENTITY_FILES: [&str; 8] = [
    "AGENTS.md",
    "SOUL.md",
    "TOOLS.md",
    "IDENTITY.md",
    "USER.md",
    "HEARTBEAT.md",
    "BOOTSTRAP.md",
    "MEMORY.md",
];

/// 单个项目实例内的指令加载状态。
#[derive(Debug, Default)]
struct State {
    claims: Mutex<HashMap<String, HashSet<String>>>,
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

/// 匹配协议文本中的 `SKILL.md` 路径（含反引号包裹与裸路径）。
static SKILL_MD_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:`([^`\n]*SKILL\.md)`|((?:\./)?(?:[\w.-]+/)+SKILL\.md))")
        .expect("skill path regex")
});

/// 从协议正文提取 `SKILL.md` 路径，去重并保持出现顺序。
fn extract_skill_md_paths(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for caps in SKILL_MD_PATH_RE.captures_iter(content) {
        let Some(path) = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str().trim()) else {
            continue;
        };
        if path.is_empty() || !seen.insert(path.to_string()) {
            continue;
        }
        out.push(path.to_string());
    }
    out
}

/// 将协议中的技能引用展开为已预加载指令或待读取的绝对路径。
fn format_required_skills_block(
    project_root: &Path,
    content: &str,
    config: &config::Config,
) -> String {
    let paths = extract_skill_md_paths(content);
    if paths.is_empty() {
        return String::new();
    }

    let canonical_root = std::fs::canonicalize(project_root).ok();
    let mut loaded_skills = Vec::new();
    let mut loaded_directories = HashSet::new();
    let mut unresolved_paths = Vec::new();

    for path in paths {
        let absolute_path = if Path::new(&path).is_absolute() {
            PathBuf::from(&path)
        } else {
            project_root.join(path.trim_start_matches("./"))
        };
        let rendered_path = normalize_str(&absolute_path);

        let Some((canonical_root, canonical_file)) =
            canonical_root.as_ref().zip(std::fs::canonicalize(&absolute_path).ok())
        else {
            unresolved_paths.push(rendered_path);
            continue;
        };
        if !canonical_file.starts_with(canonical_root) || !canonical_file.is_file() {
            unresolved_paths.push(rendered_path);
            continue;
        }

        let Some(skill_directory) = canonical_file.parent().map(Path::to_path_buf) else {
            unresolved_paths.push(rendered_path);
            continue;
        };
        if loaded_directories.contains(&skill_directory) {
            continue;
        }
        let Some(source_directory) = skill_directory.parent() else {
            unresolved_paths.push(rendered_path);
            continue;
        };

        let selected = load_skills_from_directory(
            source_directory,
            SkillLoadMode::Full,
            &config.skills.security,
        )
        .into_iter()
        .find(|skill| {
            skill
                .location
                .as_deref()
                .and_then(Path::parent)
                .and_then(|directory| std::fs::canonicalize(directory).ok())
                .is_some_and(|directory| directory == skill_directory)
        });

        if let Some(skill) = selected {
            loaded_directories.insert(skill_directory);
            loaded_skills.push(skill);
        } else {
            unresolved_paths.push(rendered_path);
        }
    }

    let mut block = String::from("\n\n<required_skills>\n");
    if !loaded_skills.is_empty() {
        block.push_str("<preloaded_skill_instructions>\n");
        block.push_str(&skills_to_prompt_with_mode(
            &loaded_skills,
            project_root,
            config::SkillsPromptInjectionMode::Full,
        ));
        block.push_str("\n</preloaded_skill_instructions>\n");
    }
    if !unresolved_paths.is_empty() {
        block.push_str("<unresolved_skill_paths>\n");
        for path in unresolved_paths {
            let _ = writeln!(block, "{path}");
        }
        block.push_str("</unresolved_skill_paths>\n");
    }
    block.push_str("</required_skills>");
    block
}

/// 返回全局指令文件候选路径。
///
/// 候选顺序体现优先级：显式配置目录优先，其次全局配置目录，最后兼容 Claude Code
/// 的用户级 `CLAUDE.md`。
fn global_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Some(dir) = flag::vibewindow_config_dir() {
        files.push(PathBuf::from(dir).join("AGENTS.md"));
    }
    files.push(global::paths().config.join("AGENTS.md"));
    if !*flag::VIBEWINDOW_DISABLE_CLAUDE_CODE_PROMPT {
        files.push(global::paths().home.join(".claude").join("CLAUDE.md"));
    }
    files
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
}

/// 查找当前系统级指令文件路径集合。
///
/// 返回规范化后的路径集合。项目内指令会按 `AGENTS.md`、`CLAUDE.md`、`CONTEXT.md`
/// 顺序选中第一类存在的文件；全局指令同样只取优先级最高的存在文件。
pub async fn system_paths() -> HashSet<String> {
    let _cfg = config::get().await;
    let mut paths = HashSet::<String>::new();

    if !flag::vibewindow_disable_project_config() {
        let start = PathBuf::from(instance::directory());
        let stop = {
            let wt = instance::worktree();
            if wt.is_empty() { None } else { Some(PathBuf::from(wt)) }
        };

        for file in FILES {
            let matches = filesystem::find_up(file, start.clone(), stop.clone()).await;
            if !matches.is_empty() {
                for p in matches {
                    paths.insert(normalize_str(p));
                }
                break;
            }
        }
    }

    for file in global_files() {
        if filesystem::exists(&file) {
            paths.insert(normalize_str(file));
            break;
        }
    }

    paths
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
    let _cfg = config::get().await;
    let paths = system_paths().await;

    let mut out = Vec::new();
    for p in paths {
        let content = std::fs::read_to_string(&p).unwrap_or_default();
        if content.trim().is_empty() {
            continue;
        }
        out.push(Resolved {
            filepath: p.clone(),
            content: format!("Instructions from: {}\n{}", p, content),
        });
    }

    out
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
/// 此路径暴露给界面。若正文提到项目内的 `SKILL.md`，会通过现有安全审计加载完整
/// 指令；无法安全加载的引用保留为绝对路径，交给模型显式读取或报告。项目配置被
/// 禁用、根目录为空、文件缺失或内容为空时返回 `None`。
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
    let filepath = normalize_str(filepath);
    let content = std::fs::read_to_string(&filepath).unwrap_or_default();
    if content.trim().is_empty() {
        return None;
    }
    let skills_block = format_required_skills_block(&root_path, &content, config);
    Some(Resolved {
        filepath: filepath.clone(),
        content: format!("Instructions from: {}\n{}{}", filepath, content, skills_block),
    })
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

    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| vw_config_types::paths::home_config_dir(home).join(format!("workspace{suffix}")))
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

/// 在指定目录查找第一份支持的指令文件。
///
/// 返回规范化路径；如果目录下没有支持的文件，返回 `None`。
pub async fn find(dir: &str) -> Option<String> {
    for file in FILES {
        let filepath = PathBuf::from(dir).join(file);
        if filesystem::exists(&filepath) {
            return Some(normalize_str(filepath));
        }
    }
    None
}

/// 已解析出的附加指令文件。
#[derive(Debug, Clone)]
pub struct Resolved {
    /// 指令文件的规范化路径。
    pub filepath: String,
    /// 带来源头的指令内容。
    pub content: String,
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
    let mut current = PathBuf::from(&target)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(&target));
    let root = filesystem::normalize_path(instance::directory());

    while current.starts_with(&root) && current != root {
        let Some(found) = find(&current.to_string_lossy().to_string()).await else {
            current = current.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| root.clone());
            continue;
        };

        if found != target
            && !system.contains(&found)
            && !already.contains(&found)
            && !is_claimed(message_id, &found).await
        {
            // claim 必须在读取前完成，防止并发解析同一消息时重复注入相同指令。
            claim(message_id, &found).await;
            let content = std::fs::read_to_string(&found).unwrap_or_default();
            if !content.trim().is_empty() {
                results.push(Resolved {
                    filepath: found.clone(),
                    content: format!("Instructions from: {}\n{}", found, content),
                });
            }
        }

        current = current.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| root.clone());
    }

    results
}
#[cfg(test)]
#[path = "instruction_tests.rs"]
mod instruction_tests;
