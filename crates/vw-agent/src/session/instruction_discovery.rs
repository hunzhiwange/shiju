//! AGENTS 指令文件的分层发现与优先级。
//!
//! 本模块是 AGENTS 来源顺序的唯一入口。顺序由低优先级到高优先级排列，令更接近当前
//! 目录的指令在提示词中后出现。

use crate::app::agent::flag;
use crate::app::agent::global;
use crate::app::agent::project::instance;
use crate::app::agent::util::filesystem;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const AGENTS_FILE: &str = "AGENTS.md";
const LOCAL_AGENTS_FILE: &str = "AGENTS.local.md";

/// 已发现 AGENTS 文件的来源层级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// 显式指定的托管配置目录。
    Managed,
    /// 用户配置目录。
    User,
    /// 项目目录中的 `AGENTS.md`。
    Project,
    /// 项目目录 `.vibewindow/AGENTS.md`。
    ProjectConfig,
    /// 项目目录 `.vibewindow/rules/*.md`。
    Rule,
    /// 项目目录中的 `AGENTS.local.md`。
    Local,
}

/// 一条有稳定来源与路径的 AGENTS 指令。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    /// 指令所属的优先级层级。
    pub kind: SourceKind,
    /// 已规范化的文件路径。
    pub filepath: PathBuf,
}

/// 发现 AGENTS 文件所需的受限根目录。
#[derive(Debug, Clone, Default)]
pub struct DiscoveryRoots {
    /// 显式托管配置目录。
    pub managed_config: Option<PathBuf>,
    /// 用户配置目录。
    pub user_config: Option<PathBuf>,
    /// 当前项目目录。
    pub current_directory: Option<PathBuf>,
    /// 项目发现不得越过的工作树根目录。
    pub worktree: Option<PathBuf>,
    /// 是否允许读取项目配置。
    pub project_config_enabled: bool,
}

/// 按托管、用户、项目根到当前目录及本地层级发现 AGENTS 指令。
pub fn discover(roots: &DiscoveryRoots) -> Vec<Source> {
    let mut sources = Vec::new();
    let mut seen = HashSet::new();

    if let Some(directory) = &roots.managed_config {
        push_file(&mut sources, &mut seen, SourceKind::Managed, directory.join(AGENTS_FILE));
    }
    if let Some(directory) = &roots.user_config {
        push_file(&mut sources, &mut seen, SourceKind::User, directory.join(AGENTS_FILE));
    }
    if roots.project_config_enabled {
        for directory in
            project_directories(roots.current_directory.as_deref(), roots.worktree.as_deref())
        {
            sources.extend(directory_sources_with_seen(&directory, &mut seen));
        }
    }

    sources
}

/// 发现单个项目目录内的 AGENTS 来源。
pub fn directory_sources(directory: &Path) -> Vec<Source> {
    let mut seen = HashSet::new();
    directory_sources_with_seen(directory, &mut seen)
}

/// 使用当前会话配置发现系统级 AGENTS 来源。
pub fn system_sources() -> Vec<Source> {
    let managed_config = flag::vibewindow_config_dir().map(PathBuf::from);
    let current_directory = (!flag::vibewindow_disable_project_config())
        .then(|| filesystem::normalize_path(instance::directory()));
    let worktree = (!flag::vibewindow_disable_project_config()).then(|| {
        let worktree = instance::worktree();
        if worktree.is_empty() { None } else { Some(filesystem::normalize_path(worktree)) }
    });

    discover(&DiscoveryRoots {
        managed_config,
        user_config: Some(global::paths().config.clone()),
        current_directory,
        worktree: worktree.flatten(),
        project_config_enabled: !flag::vibewindow_disable_project_config(),
    })
}

/// 使用请求显式传入的工作区解析 AGENTS 来源。
///
/// 会话处理器运行在独立线程时，项目 task-local 可能已经离开原异步作用域；因此模型
/// 首步不能依赖 [`instance::directory`] 猜测当前项目。显式工作区同时作为当前目录和
/// 发现边界，确保启动请求稳定读取所选项目根目录的 AGENTS 文件族。
pub fn workspace_sources(workspace: &Path) -> Vec<Source> {
    let workspace = filesystem::normalize_path(workspace);
    discover(&DiscoveryRoots {
        managed_config: flag::vibewindow_config_dir().map(PathBuf::from),
        user_config: Some(global::paths().config.clone()),
        current_directory: Some(workspace.clone()),
        worktree: Some(workspace),
        project_config_enabled: !flag::vibewindow_disable_project_config(),
    })
}

fn project_directories(current: Option<&Path>, worktree: Option<&Path>) -> Vec<PathBuf> {
    let Some(current) = current else { return Vec::new() };
    let current = filesystem::normalize_path(current);
    let worktree = worktree.map(filesystem::normalize_path);
    if worktree.as_ref().is_some_and(|root| !current.starts_with(root)) {
        return Vec::new();
    }

    let mut directories = Vec::new();
    let mut directory = current;
    loop {
        directories.push(directory.clone());
        if worktree.as_ref().is_some_and(|root| root == &directory) {
            break;
        }
        let Some(parent) = directory.parent().map(Path::to_path_buf) else { break };
        if parent == directory {
            break;
        }
        directory = parent;
    }
    directories.reverse();
    directories
}

fn directory_sources_with_seen(directory: &Path, seen: &mut HashSet<PathBuf>) -> Vec<Source> {
    let mut sources = Vec::new();
    push_file(&mut sources, seen, SourceKind::Project, directory.join(AGENTS_FILE));
    let vibewindow = directory.join(".vibewindow");
    push_file(&mut sources, seen, SourceKind::ProjectConfig, vibewindow.join(AGENTS_FILE));

    let mut rules = std::fs::read_dir(vibewindow.join("rules"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().is_some_and(|extension| extension == "md")
        })
        .collect::<Vec<_>>();
    rules.sort();
    for rule in rules {
        push_file(&mut sources, seen, SourceKind::Rule, rule);
    }
    push_file(&mut sources, seen, SourceKind::Local, directory.join(LOCAL_AGENTS_FILE));
    sources
}

fn push_file(
    sources: &mut Vec<Source>,
    seen: &mut HashSet<PathBuf>,
    kind: SourceKind,
    filepath: PathBuf,
) {
    let filepath = filesystem::normalize_path(filepath);
    if filepath.is_file() && seen.insert(filepath.clone()) {
        sources.push(Source { kind, filepath });
    }
}

#[cfg(test)]
#[path = "instruction_discovery_tests.rs"]
mod instruction_discovery_tests;
