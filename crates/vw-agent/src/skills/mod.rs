//! # 技能模块（Skills Module）
//!
//! 本模块提供了代理技能的加载、初始化和管理功能。
//!
//! ## 模块职责
//!
//! - **技能加载**：从工作空间目录和 OpenSkills 仓库加载技能定义
//! - **技能发现**：自动发现并列出可用的技能
//! - **技能管理**：提供技能目录初始化、安装、卸载等管理操作
//! - **技能审计**：审计技能的安全性、合规性等
//!
//! ## 主要组件
//!
//! - [`load_skills`]：从工作空间加载所有技能
//! - [`load_skills_with_config`]：使用运行时配置加载技能
//! - [`init_skills_dir`]：初始化技能目录
//! - [`handle_command`]：处理技能相关的 CLI 命令
//!
//! ## 子模块
//!
//! - `audit`：技能安全审计
//! - `cli`：命令行接口处理
//! - `init`：技能目录初始化
//! - `installer`：技能安装器
//! - `loader`：技能加载器
//! - `open_skills`：OpenSkills 集成
//! - `policy`：技能策略管理
//! - `prompt`：技能提示词生成
//! - `source`：技能源管理
//! - `types`：技能类型定义

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use vw_config_types::skills::SkillsDirectoryProvider;

mod audit;
mod cli;
mod discovery;
mod init;
mod installer;
mod invoked;
mod listing;
mod loader;
pub(crate) mod marketplace;
mod open_skills;
mod policy;
mod prompt;
mod session_state;
pub(crate) mod shiju;
mod source;
mod types;

pub(crate) use invoked::{InvokedSkillRecord, InvokedSkillReplay};
pub(crate) use loader::{load_skills_from_directory, read_markdown_skill_metadata};
pub use prompt::{skills_to_prompt, skills_to_prompt_with_mode};
pub(crate) use session_state::SkillListingSessionState;
pub(crate) use types::SkillLoadMode;
pub use types::{Skill, SkillTool};

const SKILL_DISABLED_MARKER_FILE: &str = "SKILL.disabled";
const COMMON_PROJECT_SKILL_SOURCE_PATHS: [&str; 2] = ["skills", ".agents/skills"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalSkillSourceKind {
    Workspace,
    Ancestor,
    Global,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalSkillSourceDir {
    pub(crate) kind: LocalSkillSourceKind,
    pub(crate) path: PathBuf,
}

pub(crate) fn local_skill_disabled_marker_path(skill_dir: &Path) -> PathBuf {
    skill_dir.join(SKILL_DISABLED_MARKER_FILE)
}

pub(crate) fn is_local_skill_disabled(skill_dir: &Path) -> bool {
    local_skill_disabled_marker_path(skill_dir).is_file()
}

pub(crate) fn discover_local_skill_source_dirs(
    workspace_dir: Option<&Path>,
) -> Vec<LocalSkillSourceDir> {
    discover_local_skill_source_dirs_with_provider(
        workspace_dir,
        SkillsDirectoryProvider::Vibewindow,
    )
}

pub(crate) fn discover_local_skill_source_dirs_with_provider(
    workspace_dir: Option<&Path>,
    directory_provider: SkillsDirectoryProvider,
) -> Vec<LocalSkillSourceDir> {
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();
    let home_dir = std::env::var_os("HOME").map(PathBuf::from);

    if let Some(workspace_dir) = workspace_dir {
        let workspace_dir =
            workspace_dir.canonicalize().unwrap_or_else(|_| workspace_dir.to_path_buf());

        for relative_path in workspace_skill_source_paths(directory_provider)
            .into_iter()
            .chain(COMMON_PROJECT_SKILL_SOURCE_PATHS)
        {
            push_skill_source_dir(
                &mut dirs,
                &mut seen,
                LocalSkillSourceKind::Workspace,
                workspace_dir.join(relative_path),
            );
        }

        let stop_dir = discover_ancestor_scan_stop(&workspace_dir, home_dir.as_deref());
        if stop_dir.as_ref() != Some(&workspace_dir) {
            let mut current = workspace_dir.parent().map(Path::to_path_buf);
            while let Some(dir) = current {
                if home_dir.as_ref().is_some_and(|home| dir == home.as_path()) {
                    break;
                }

                for relative_path in ancestor_skill_source_paths(directory_provider)
                    .into_iter()
                    .chain(COMMON_PROJECT_SKILL_SOURCE_PATHS)
                {
                    push_skill_source_dir(
                        &mut dirs,
                        &mut seen,
                        LocalSkillSourceKind::Ancestor,
                        dir.join(relative_path),
                    );
                }

                if stop_dir.as_ref().is_some_and(|stop| dir == stop.as_path()) {
                    break;
                }

                current = dir.parent().map(Path::to_path_buf);
            }
        }
    }

    if let Some(home_dir) = home_dir {
        if directory_provider == SkillsDirectoryProvider::Vibewindow {
            push_skill_source_dir(
                &mut dirs,
                &mut seen,
                LocalSkillSourceKind::Global,
                vw_config_types::paths::home_config_dir(&home_dir).join("skills"),
            );
            push_skill_source_dir(
                &mut dirs,
                &mut seen,
                LocalSkillSourceKind::Global,
                home_dir.join(".skills"),
            );
        } else {
            for relative_path in global_skill_source_paths(directory_provider) {
                push_skill_source_dir(
                    &mut dirs,
                    &mut seen,
                    LocalSkillSourceKind::Global,
                    home_dir.join(relative_path),
                );
            }
        }
    }

    dirs
}

pub(crate) fn workspace_skills_dir(
    workspace_dir: &Path,
    directory_provider: SkillsDirectoryProvider,
) -> PathBuf {
    workspace_dir.join(primary_workspace_skill_source_path(directory_provider))
}

fn workspace_skill_source_paths(directory_provider: SkillsDirectoryProvider) -> Vec<&'static str> {
    match directory_provider {
        SkillsDirectoryProvider::Vibewindow => vec![".vibewindow/skills", "skills"],
        SkillsDirectoryProvider::Codex => vec![".codex/skills", ".agents/skills"],
        SkillsDirectoryProvider::Claude => vec![".claude/skills"],
        SkillsDirectoryProvider::Cursor => vec![".cursor/skills"],
    }
}

fn ancestor_skill_source_paths(directory_provider: SkillsDirectoryProvider) -> Vec<&'static str> {
    match directory_provider {
        SkillsDirectoryProvider::Vibewindow => vec![".vibewindow/skills"],
        SkillsDirectoryProvider::Codex => vec![".codex/skills", ".agents/skills"],
        SkillsDirectoryProvider::Claude => vec![".claude/skills"],
        SkillsDirectoryProvider::Cursor => vec![".cursor/skills"],
    }
}

fn global_skill_source_paths(directory_provider: SkillsDirectoryProvider) -> Vec<&'static str> {
    match directory_provider {
        // `~/.skills` 是比 VibeWindow 配置目录更通用的用户级技能目录。
        SkillsDirectoryProvider::Vibewindow => vec![".vibewindow/skills", ".skills"],
        SkillsDirectoryProvider::Codex => vec![".codex/skills", ".agents/skills"],
        SkillsDirectoryProvider::Claude => vec![".claude/skills"],
        SkillsDirectoryProvider::Cursor => vec![".cursor/skills"],
    }
}

fn primary_workspace_skill_source_path(
    directory_provider: SkillsDirectoryProvider,
) -> &'static str {
    match directory_provider {
        SkillsDirectoryProvider::Vibewindow => ".vibewindow/skills",
        SkillsDirectoryProvider::Codex => ".codex/skills",
        SkillsDirectoryProvider::Claude => ".claude/skills",
        SkillsDirectoryProvider::Cursor => ".cursor/skills",
    }
}

fn push_skill_source_dir(
    dirs: &mut Vec<LocalSkillSourceDir>,
    seen: &mut HashSet<PathBuf>,
    kind: LocalSkillSourceKind,
    path: PathBuf,
) {
    let realpath = path.canonicalize().unwrap_or_else(|_| path.clone());
    if path.is_dir() && seen.insert(realpath) {
        dirs.push(LocalSkillSourceDir { kind, path });
    }
}

fn discover_ancestor_scan_stop(workspace_dir: &Path, home_dir: Option<&Path>) -> Option<PathBuf> {
    let mut current = Some(workspace_dir);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        if home_dir.is_some_and(|home| dir == home) {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

fn extend_unique_skills(target: &mut Vec<Skill>, seen: &mut HashSet<String>, items: Vec<Skill>) {
    for skill in items {
        if seen.insert(skill.name.clone()) {
            target.push(skill);
        }
    }
}

/// 从工作空间技能目录加载所有技能
///
/// 此函数会从指定工作空间的 `skills` 目录中加载所有技能定义。
/// 使用默认的完整加载模式（`SkillLoadMode::Full`）加载技能的完整信息。
///
/// # 参数
///
/// * `workspace_dir` - 工作空间根目录的路径
///
/// # 返回值
///
/// 返回加载的所有技能列表（`Vec<Skill>`）
///
/// # 示例
///
/// ```rust,no_run
/// use std::path::Path;
/// use vibewindow::app::agent::skills::load_skills;
///
/// let workspace = Path::new("/path/to/workspace");
/// let skills = load_skills(workspace);
/// println!("已加载 {} 个技能", skills.len());
/// ```
pub fn load_skills(workspace_dir: &Path) -> Vec<Skill> {
    load_skills_with_open_skills_config(workspace_dir, None, None, types::SkillLoadMode::Full)
}

/// 使用运行时配置加载技能（推荐在运行时使用）
///
/// 此函数根据提供的配置对象加载技能，包括 OpenSkills 的启用状态、
/// OpenSkills 目录路径以及提示词注入模式等配置项。
///
/// # 参数
///
/// * `workspace_dir` - 工作空间根目录的路径
/// * `config` - 应用配置对象，包含技能相关的配置项
///
/// # 返回值
///
/// 返回加载的所有技能列表（`Vec<Skill>`）
///
/// # 示例
///
/// ```rust,no_run
/// use std::path::Path;
/// use vibewindow::app::agent::config::Config;
/// use vibewindow::app::agent::skills::load_skills_with_config;
///
/// let workspace = Path::new("/path/to/workspace");
/// let config = Config::load(workspace).unwrap();
/// let skills = load_skills_with_config(workspace, &config);
/// println!("已加载 {} 个技能", skills.len());
/// ```
pub fn load_skills_with_config(
    workspace_dir: &Path,
    config: &crate::app::agent::config::Config,
) -> Vec<Skill> {
    load_skills_with_open_skills_config_and_provider(
        workspace_dir,
        Some(config.skills.open_skills_enabled),
        config.skills.open_skills_dir.as_deref(),
        types::SkillLoadMode::from_prompt_mode(config.skills.prompt_injection_mode),
        config.skills.directory_provider,
        &config.skills.security,
        Some(config),
    )
}

/// 使用运行时配置加载完整技能内容。
///
/// skill 工具按需加载时需要完整指令；这里不受提示词注入模式影响。
pub(crate) fn load_skills_full_with_config(
    workspace_dir: &Path,
    config: &crate::app::agent::config::Config,
) -> Vec<Skill> {
    load_skills_with_open_skills_config_and_provider(
        workspace_dir,
        Some(config.skills.open_skills_enabled),
        config.skills.open_skills_dir.as_deref(),
        types::SkillLoadMode::Full,
        config.skills.directory_provider,
        &config.skills.security,
        Some(config),
    )
}

/// 使用 OpenSkills 配置加载技能（内部实现）
///
/// 这是技能加载的核心实现函数，会根据配置决定是否加载 OpenSkills 仓库中的技能，
/// 然后加载工作空间本地的技能。两个来源的技能会被合并返回。
///
/// # 参数
///
/// * `workspace_dir` - 工作空间根目录的路径
/// * `config_open_skills_enabled` - 是否启用 OpenSkills（`None` 表示使用默认值）
/// * `config_open_skills_dir` - OpenSkills 目录路径（`None` 表示使用默认位置）
/// * `load_mode` - 技能加载模式（完整模式、轻量模式等）
///
/// # 返回值
///
/// 返回加载的所有技能列表（`Vec<Skill>`），包含 OpenSkills 和工作空间技能
///
/// # 加载顺序
///
/// 1. 优先加载当前工作区与祖先目录中的本地技能
/// 2. 然后补充全局 VibeWindow 配置目录中的 `skills` 与 `~/.skills` 技能
/// 3. 最后在启用时追加 OpenSkills 社区仓库技能
/// 4. 同名技能按前面的来源优先，后续来源会被去重跳过
pub(crate) fn load_skills_with_open_skills_config(
    workspace_dir: &Path,
    config_open_skills_enabled: Option<bool>,
    config_open_skills_dir: Option<&str>,
    load_mode: types::SkillLoadMode,
) -> Vec<Skill> {
    load_skills_with_open_skills_config_and_provider(
        workspace_dir,
        config_open_skills_enabled,
        config_open_skills_dir,
        load_mode,
        SkillsDirectoryProvider::Vibewindow,
        &vw_config_types::skills::SkillSecurityConfig::default(),
        None,
    )
}

fn load_skills_with_open_skills_config_and_provider(
    workspace_dir: &Path,
    config_open_skills_enabled: Option<bool>,
    config_open_skills_dir: Option<&str>,
    load_mode: types::SkillLoadMode,
    directory_provider: SkillsDirectoryProvider,
    security: &vw_config_types::skills::SkillSecurityConfig,
    config: Option<&crate::app::agent::config::Config>,
) -> Vec<Skill> {
    let mut skills = Vec::new();
    let mut seen = HashSet::new();

    extend_unique_skills(
        &mut skills,
        &mut seen,
        load_workspace_skills(workspace_dir, load_mode, directory_provider, security),
    );

    if let Some(config) = config {
        extend_unique_skills(
            &mut skills,
            &mut seen,
            crate::plugins::load_plugin_skills(workspace_dir, config, load_mode),
        );
    }

    if let Some(open_skills_dir) = open_skills::resolve_local_open_skills_repo(
        config_open_skills_enabled,
        config_open_skills_dir,
    ) {
        extend_unique_skills(
            &mut skills,
            &mut seen,
            open_skills::load_open_skills(&open_skills_dir, load_mode, security),
        );
    }
    open_skills::schedule_open_skills_sync(config_open_skills_enabled, config_open_skills_dir);

    skills
}

/// 从工作空间加载本地技能（内部辅助函数）
///
/// 从指定工作空间的 `skills` 子目录中加载所有技能定义。
///
/// # 参数
///
/// * `workspace_dir` - 工作空间根目录的路径
/// * `load_mode` - 技能加载模式
///
/// # 返回值
///
/// 返回工作空间中的技能列表（`Vec<Skill>`）
fn load_workspace_skills(
    workspace_dir: &Path,
    load_mode: types::SkillLoadMode,
    directory_provider: SkillsDirectoryProvider,
    security: &vw_config_types::skills::SkillSecurityConfig,
) -> Vec<Skill> {
    discover_local_skill_source_dirs_with_provider(Some(workspace_dir), directory_provider)
        .into_iter()
        .flat_map(|source| loader::load_skills_from_directory(&source.path, load_mode, security))
        .collect()
}

/// 加载委托代理可默认使用的项目与父级技能。
pub(crate) fn load_project_skills_with_config(
    workspace_dir: &Path,
    config: &crate::app::agent::config::Config,
) -> Vec<Skill> {
    let mut skills = Vec::new();
    let mut seen = HashSet::new();
    let load_mode = types::SkillLoadMode::from_prompt_mode(config.skills.prompt_injection_mode);

    for source in discover_local_skill_source_dirs_with_provider(
        Some(workspace_dir),
        config.skills.directory_provider,
    )
    .into_iter()
    .filter(|source| {
        matches!(source.kind, LocalSkillSourceKind::Workspace | LocalSkillSourceKind::Ancestor)
    }) {
        extend_unique_skills(
            &mut skills,
            &mut seen,
            loader::load_skills_from_directory(&source.path, load_mode, &config.skills.security),
        );
    }

    skills
}
/// 获取技能目录路径
///
/// 返回指定工作空间下的技能目录路径（`{workspace_dir}/skills`）。
///
/// # 参数
///
/// * `workspace_dir` - 工作空间根目录的路径
///
/// # 返回值
///
/// 返回技能目录的完整路径（`PathBuf`）
///
/// # 示例
///
/// ```rust,no_run
/// use std::path::Path;
/// use vibewindow::app::agent::skills::skills_dir;
///
/// let workspace = Path::new("/path/to/workspace");
/// let skills_path = skills_dir(workspace);
/// println!("技能目录：{}", skills_path.display());
/// ```
pub fn skills_dir(workspace_dir: &Path) -> PathBuf {
    workspace_dir.join("skills")
}

/// 初始化技能目录
///
/// 在指定的工作空间中创建技能目录结构。如果目录已存在，则不会执行任何操作。
/// 此函数会创建必要的目录结构和默认配置文件。
///
/// # 参数
///
/// * `workspace_dir` - 工作空间根目录的路径
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回错误信息
///
/// # 错误
///
/// 如果无法创建目录或写入文件，将返回相应的 IO 错误
///
/// # 示例
///
/// ```rust,no_run
/// use std::path::Path;
/// use vibewindow::app::agent::skills::init_skills_dir;
///
/// let workspace = Path::new("/path/to/workspace");
/// init_skills_dir(workspace).expect("初始化技能目录失败");
/// ```
pub fn init_skills_dir(workspace_dir: &Path) -> anyhow::Result<()> {
    init::init_skills_dir(workspace_dir)
}

/// 处理技能相关的 CLI 命令
///
/// 此函数是技能命令行接口的入口点，用于处理各种技能管理命令，
/// 如安装、卸载、列出、更新等操作。
///
/// # 参数
///
/// * `command` - 技能命令枚举，指定要执行的操作
/// * `config` - 应用配置对象，提供必要的运行时配置
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回错误信息
///
/// # 支持的命令
///
/// - 列出技能
/// - 安装技能
/// - 卸载技能
/// - 更新技能
/// - 显示技能详情
///
/// # 示例
///
/// ```rust,no_run
/// use vibewindow::app::agent::skill::SkillCommands;
/// use vibewindow::app::agent::config::Config;
/// use vibewindow::app::agent::skills::handle_command;
/// use std::path::Path;
///
/// let workspace = Path::new("/path/to/workspace");
/// let config = Config::load(workspace).unwrap();
/// let command = SkillCommands::List;
/// handle_command(command, &config).expect("命令执行失败");
/// ```
pub fn handle_command(
    command: crate::app::agent::skill::SkillCommands,
    config: &crate::app::agent::config::Config,
) -> anyhow::Result<()> {
    cli::handle_command(command, config)
}

// ============================================================================
// 测试模块
// ============================================================================

/// 符号链接相关测试
#[cfg(test)]
mod symlink_tests;

/// 技能模块集成测试
#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;

#[cfg(test)]
#[path = "discovery_tests.rs"]
mod discovery_tests;

#[cfg(test)]
#[path = "listing_tests.rs"]
mod listing_tests;

#[cfg(test)]
#[path = "invoked_tests.rs"]
mod invoked_tests;
#[cfg(test)]
#[path = "session_state_tests.rs"]
mod session_state_tests;
