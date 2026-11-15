//! Instruction discovery, expansion and per-project loading state.
use agent_util::filesystem;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;
pub mod instruction_discovery;
pub mod instruction_cache;
pub mod instruction_include;
pub mod instruction_rules;
use agent_security::SecurityPolicy;
const WORKSPACE_IDENTITY_FILES: [&str; 7] =
    ["AGENTS.md", "SOUL.md", "TOOLS.md", "IDENTITY.md", "USER.md", "HEARTBEAT.md", "MEMORY.md"];

/// 单个项目实例内的指令加载状态。
#[derive(Debug, Default)]
pub struct State {
    claims: Mutex<HashMap<String, HashSet<String>>>,
    cache: Mutex<instruction_cache::InstructionCache>,
}

/// 将路径规范化为可比较的字符串。
pub fn normalize_str(p: impl AsRef<Path>) -> String {
    filesystem::normalize_path(p).to_string_lossy().to_string()
}

#[cfg(target_arch = "wasm32")]
/// wasm 架构下的 URL 指令占位加载器。
pub async fn fetch_url(_url: &str) -> String {
    String::new()
}

#[cfg(not(target_arch = "wasm32"))]
/// 在非 wasm 环境中读取远程 URL 指令内容。
///
/// 网络请求放入阻塞线程并设置短超时；失败时返回空字符串，避免指令加载阻塞主会话。
pub async fn fetch_url(url: &str) -> String {
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
        |home| config_runtime_paths::home_config_dir(home).join(format!("workspace{suffix}")),
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

/// 在指定目录查找优先级最高的 AGENTS 指令文件。
///
/// 返回规范化路径；如果目录下没有支持的文件，返回 `None`。
pub async fn find(dir: &str) -> Option<String> {
    instruction_discovery::directory_sources(Path::new(dir))
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

pub fn resolved_file(
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

#[cfg(test)]
#[path = "instruction_cache_tests.rs"]
mod instruction_cache_tests;

#[cfg(test)]
#[path = "instruction_rules_tests.rs"]
mod instruction_rules_tests;

/// Resolve additional instructions using the host's project state and authorized policy.
pub async fn resolve(
    state: &State,
    system: &HashSet<String>,
    already: &HashSet<String>,
    filepath: &str,
    message_id: &str,
    root: PathBuf,
    policy: &SecurityPolicy,
) -> Vec<Resolved> {
    let mut results = Vec::new();
    let target = normalize_str(filepath);
    let mut current = PathBuf::from(&target)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from(&target));

    while current.starts_with(&root) {
        for source in instruction_discovery::directory_sources(&current) {
            let found = normalize_str(&source.filepath);
            if found != target
                && !system.contains(&found)
                && !already.contains(&found)
                && !state.is_claimed(message_id, &found).await
            {
                // claim 必须在读取前完成，防止并发解析同一消息时重复注入相同指令。
                state.claim(message_id, &found).await;
                if source.kind == instruction_discovery::SourceKind::Rule {
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
                    let mut cache = state.cache.lock().await;
                    let (resolved, _) = cache.load(&root, message_id, &source.filepath, || {
                        resolved_file(&source.filepath, policy, Vec::new())
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
impl State {
    pub async fn is_claimed(&self, message_id: &str, filepath: &str) -> bool {
        let claims = self.claims.lock().await;
        claims.get(message_id).is_some_and(|s| s.contains(filepath))
    }
    pub async fn claim(&self, message_id: &str, filepath: &str) {
        self.claims.lock().await.entry(message_id.to_string()).or_default().insert(filepath.to_string());
    }
    pub async fn clear(&self, workspace: &Path, message_id: &str) {
        self.claims.lock().await.remove(message_id);
        self.cache.lock().await.clear_session(workspace, message_id);
    }
    pub async fn reload_after_compaction(&self, workspace: &Path, message_id: &str) {
        self.claims.lock().await.remove(message_id);
        self.cache.lock().await.invalidate_for_compaction(workspace, message_id);
    }
}

#[cfg(test)]
#[path = "instruction_include_tests.rs"]
mod instruction_include_tests;
