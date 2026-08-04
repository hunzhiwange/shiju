//! 开放技能仓库管理模块
//!
//! 本模块负责管理开源技能仓库（open-skills）的生命周期，包括：
//! - 检测和解析开放技能功能的启用状态
//! - 解析和确定技能仓库的存储路径
//! - 自动克隆和同步远程技能仓库
//! - 加载并审计开放技能定义文件
//!
//! # 配置优先级
//!
//! 开放技能功能的启用/禁用状态按以下优先级解析：
//! 1. 环境变量 `VIBEWINDOW_OPEN_SKILLS_ENABLED`
//! 2. 配置文件中的 `open_skills_enabled` 字段
//!
//! 技能仓库路径按以下优先级解析：
//! 1. 环境变量 `VIBEWINDOW_OPEN_SKILLS_DIR`
//! 2. 配置文件中的 `open_skills_dir` 字段
//! 3. 用户主目录下的 `open-skills` 文件夹

use crate::app::agent::shell::git_std_command;
use crate::app::agent::skills::loader::load_skills_from_directory;
use crate::app::agent::skills::types::SkillLoadMode;
use directories::UserDirs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// 开放技能仓库的远程 URL 地址
///
/// 该 URL 指向 GitHub 上的官方开放技能仓库，
/// 用于在本地不存在仓库时自动克隆，或在需要时拉取更新。
const OPEN_SKILLS_REPO_URL: &str = "https://github.com/besoeasy/open-skills";

/// 同步标记文件的名称
///
/// 该文件用于记录最后一次成功同步的时间戳，
/// 通过检查该文件的修改时间来判断是否需要进行新的同步操作。
const OPEN_SKILLS_SYNC_MARKER: &str = ".vibewindow-open-skills-sync";

/// 同步间隔时间（秒）
///
/// 定义两次自动同步之间的最小间隔时间。
/// 默认值为 7 天（604800 秒），以避免过于频繁的网络请求。
const OPEN_SKILLS_SYNC_INTERVAL_SECS: u64 = 60 * 60 * 24 * 7;

/// 解析开放技能启用状态的字符串值
///
/// 将原始字符串转换为布尔值，支持多种常见的布尔表示格式：
/// - `true` 值：`"1"`, `"true"`, `"yes"`, `"on"`
/// - `false` 值：`"0"`, `"false"`, `"no"`, `"off"`
///
/// # 参数
///
/// - `raw`: 原始字符串值（不区分大小写，会自动去除首尾空白）
///
/// # 返回值
///
/// - `Some(true)`: 字符串表示启用
/// - `Some(false)`: 字符串表示禁用
/// - `None`: 字符串无法识别为有效的布尔值
fn parse_open_skills_enabled(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

/// 从多个配置源综合判断开放技能功能是否启用
///
/// 按优先级顺序检查配置源：
/// 1. 环境变量覆盖（如果提供且有效）
/// 2. 配置文件中的设置（如果环境变量未提供或无效）
///
/// # 参数
///
/// - `config_open_skills_enabled`: 配置文件中的启用设置（可能为 `None`）
/// - `env_override`: 环境变量的原始值（可能为 `None`）
///
/// # 返回值
///
/// 返回 `true` 表示开放技能功能已启用，`false` 表示禁用。
///
/// # 示例
///
/// ```ignore
/// // 环境变量优先
/// let enabled = open_skills_enabled_from_sources(Some(false), Some("true"));
/// assert_eq!(enabled, true);
///
/// // 环境变量无效时使用配置文件值
/// let enabled = open_skills_enabled_from_sources(Some(true), Some("invalid"));
/// assert_eq!(enabled, true);
///
/// // 均未设置时默认禁用
/// let enabled = open_skills_enabled_from_sources(None, None);
/// assert_eq!(enabled, false);
/// ```
pub(crate) fn open_skills_enabled_from_sources(
    config_open_skills_enabled: Option<bool>,
    env_override: Option<&str>,
) -> bool {
    if let Some(raw) = env_override {
        if let Some(enabled) = parse_open_skills_enabled(raw) {
            return enabled;
        }
        if !raw.trim().is_empty() {
            tracing::warn!(
                "Ignoring invalid VIBEWINDOW_OPEN_SKILLS_ENABLED (valid: 1|0|true|false|yes|no|on|off)"
            );
        }
    }

    config_open_skills_enabled.unwrap_or(false)
}

/// 判断开放技能功能是否启用（便捷包装函数）
///
/// 自动从环境变量 `VIBEWINDOW_OPEN_SKILLS_ENABLED` 读取覆盖值，
/// 并结合配置文件设置进行综合判断。
///
/// # 参数
///
/// - `config_open_skills_enabled`: 配置文件中的启用设置（可能为 `None`）
///
/// # 返回值
///
/// 返回 `true` 表示开放技能功能已启用，`false` 表示禁用。
fn open_skills_enabled(config_open_skills_enabled: Option<bool>) -> bool {
    let env_override = std::env::var("VIBEWINDOW_OPEN_SKILLS_ENABLED").ok();
    open_skills_enabled_from_sources(config_open_skills_enabled, env_override.as_deref())
}

/// 从多个配置源解析开放技能仓库的存储路径
///
/// 按优先级顺序检查路径配置：
/// 1. 环境变量 `VIBEWINDOW_OPEN_SKILLS_DIR`（如果提供且非空）
/// 2. 配置文件中的 `open_skills_dir` 字段（如果提供且非空）
/// 3. 用户主目录下的 `open-skills` 文件夹（作为默认值）
///
/// # 参数
///
/// - `env_dir`: 环境变量指定的路径（可能为 `None`）
/// - `config_dir`: 配置文件指定的路径（可能为 `None`）
/// - `home_dir`: 用户主目录路径（可能为 `None`）
///
/// # 返回值
///
/// 返回解析后的路径，如果所有源均为空或无效则返回 `None`。
///
/// # 示例
///
/// ```ignore
/// use std::path::Path;
///
/// // 环境变量优先
/// let path = resolve_open_skills_dir_from_sources(
///     Some("/custom/path"),
///     Some("/config/path"),
///     Some(Path::new("/home/user"))
/// );
/// assert_eq!(path, Some(PathBuf::from("/custom/path")));
///
/// // 使用配置文件值
/// let path = resolve_open_skills_dir_from_sources(
///     None,
///     Some("/config/path"),
///     Some(Path::new("/home/user"))
/// );
/// assert_eq!(path, Some(PathBuf::from("/config/path")));
///
/// // 使用默认值
/// let path = resolve_open_skills_dir_from_sources(
///     None,
///     None,
///     Some(Path::new("/home/user"))
/// );
/// assert_eq!(path, Some(PathBuf::from("/home/user/open-skills")));
/// ```
pub(crate) fn resolve_open_skills_dir_from_sources(
    env_dir: Option<&str>,
    config_dir: Option<&str>,
    home_dir: Option<&Path>,
) -> Option<PathBuf> {
    let parse_dir = |raw: &str| {
        let trimmed = raw.trim();
        if trimmed.is_empty() { None } else { Some(PathBuf::from(trimmed)) }
    };

    if let Some(env_dir) = env_dir.and_then(parse_dir) {
        return Some(env_dir);
    }
    if let Some(config_dir) = config_dir.and_then(parse_dir) {
        return Some(config_dir);
    }
    home_dir.map(|home| home.join("open-skills"))
}

/// 解析开放技能仓库的存储路径（便捷包装函数）
///
/// 自动从环境变量 `VIBEWINDOW_OPEN_SKILLS_DIR` 读取覆盖值，
/// 并结合配置文件设置和用户主目录进行路径解析。
///
/// # 参数
///
/// - `config_open_skills_dir`: 配置文件指定的路径（可能为 `None`）
///
/// # 返回值
///
/// 返回解析后的路径。如果环境变量和配置文件均未指定，
/// 则尝试使用用户主目录下的 `open-skills` 文件夹。
fn resolve_open_skills_dir(config_open_skills_dir: Option<&str>) -> Option<PathBuf> {
    let env_dir = std::env::var("VIBEWINDOW_OPEN_SKILLS_DIR").ok();
    let home_dir = UserDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
    resolve_open_skills_dir_from_sources(
        env_dir.as_deref(),
        config_open_skills_dir,
        home_dir.as_deref(),
    )
}

/// 确保开放技能仓库可用
///
/// 该函数是开放技能管理的主入口，负责：
/// 1. 检查开放技能功能是否启用（如未启用则直接返回 `None`）
/// 2. 解析技能仓库的存储路径
/// 3. 如果仓库不存在，自动克隆远程仓库
/// 4. 如果仓库存在且超过同步间隔，尝试拉取最新更新
/// 5. 记录同步时间戳以避免频繁同步
///
/// # 参数
///
/// - `config_open_skills_enabled`: 配置文件中的启用设置（可能为 `None`）
/// - `config_open_skills_dir`: 配置文件指定的仓库路径（可能为 `None`）
///
/// # 返回值
///
/// - `Some(PathBuf)`: 技能仓库的本地路径
/// - `None`: 功能未启用或发生严重错误
///
/// # 同步策略
///
/// - 使用浅克隆（`--depth 1`）以减少下载量
/// - 使用快进合并（`--ff-only`）以避免合并冲突
/// - 同步失败时继续使用本地副本，仅记录警告
///
/// # 示例
///
/// ```ignore
/// if let Some(repo_dir) = ensure_open_skills_repo(Some(true), None) {
///     println!("技能仓库已就绪：{}", repo_dir.display());
/// }
/// ```
pub(crate) fn ensure_open_skills_repo(
    config_open_skills_enabled: Option<bool>,
    config_open_skills_dir: Option<&str>,
) -> Option<PathBuf> {
    if !open_skills_enabled(config_open_skills_enabled) {
        return None;
    }

    let repo_dir = resolve_open_skills_dir(config_open_skills_dir)?;

    if !repo_dir.exists() {
        if !clone_open_skills_repo(&repo_dir) {
            return None;
        }
        let _ = mark_open_skills_synced(&repo_dir);
        return Some(repo_dir);
    }

    if should_sync_open_skills(&repo_dir) {
        if pull_open_skills_repo(&repo_dir) {
            let _ = mark_open_skills_synced(&repo_dir);
        } else {
            tracing::warn!(
                "open-skills update failed; using local copy from {}",
                repo_dir.display()
            );
        }
    }

    Some(repo_dir)
}

/// 克隆开放技能仓库
///
/// 从远程 URL 克隆技能仓库到指定的本地路径。
/// 使用浅克隆（`--depth 1`）以最小化下载量。
///
/// # 参数
///
/// - `repo_dir`: 目标仓库路径
///
/// # 返回值
///
/// - `true`: 克隆成功
/// - `false`: 克隆失败（创建目录失败或 git 命令执行失败）
fn clone_open_skills_repo(repo_dir: &Path) -> bool {
    if let Some(parent) = repo_dir.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!(
                "failed to create open-skills parent directory {}: {err}",
                parent.display()
            );
            return false;
        }
    }

    let output = git_std_command()
        .args(["clone", "--depth", "1", OPEN_SKILLS_REPO_URL])
        .arg(repo_dir)
        .output();

    match output {
        Ok(result) if result.status.success() => {
            tracing::info!("initialized open-skills at {}", repo_dir.display());
            true
        }
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            tracing::warn!("failed to clone open-skills: {stderr}");
            false
        }
        Err(err) => {
            tracing::warn!("failed to run git clone for open-skills: {err}");
            false
        }
    }
}

/// 拉取开放技能仓库的更新
///
/// 执行 `git pull --ff-only` 命令以获取远程仓库的最新更新。
/// 使用快进合并策略以避免产生合并冲突。
///
/// # 参数
///
/// - `repo_dir`: 本地仓库路径
///
/// # 返回值
///
/// - `true`: 拉取成功，或目录不是 git 仓库（静默跳过）
/// - `false`: 拉取失败
///
/// # 特殊行为
///
/// 如果指定目录不是 git 仓库（例如用户通过环境变量指向了普通目录），
/// 函数会静默返回 `true`，继续使用该目录而不会报错。
fn pull_open_skills_repo(repo_dir: &Path) -> bool {
    if !repo_dir.join(".git").exists() {
        return true;
    }

    let output = git_std_command().arg("-C").arg(repo_dir).args(["pull", "--ff-only"]).output();

    match output {
        Ok(result) if result.status.success() => true,
        Ok(result) => {
            let stderr = String::from_utf8_lossy(&result.stderr);
            tracing::warn!("failed to pull open-skills updates: {stderr}");
            false
        }
        Err(err) => {
            tracing::warn!("failed to run git pull for open-skills: {err}");
            false
        }
    }
}

/// 判断是否应该同步开放技能仓库
///
/// 通过检查同步标记文件的修改时间来判断是否需要进行新的同步。
/// 如果标记文件不存在或无法读取，默认返回 `true`（需要同步）。
///
/// # 参数
///
/// - `repo_dir`: 本地仓库路径
///
/// # 返回值
///
/// - `true`: 距离上次同步已超过配置的间隔时间，需要同步
/// - `false`: 尚未超过间隔时间，无需同步
///
/// # 同步间隔
///
/// 同步间隔由常量 [`OPEN_SKILLS_SYNC_INTERVAL_SECS`] 定义，默认为 7 天。
fn should_sync_open_skills(repo_dir: &Path) -> bool {
    let marker = repo_dir.join(OPEN_SKILLS_SYNC_MARKER);
    let Ok(metadata) = std::fs::metadata(marker) else {
        return true;
    };
    let Ok(modified_at) = metadata.modified() else {
        return true;
    };
    let Ok(age) = SystemTime::now().duration_since(modified_at) else {
        return true;
    };

    age >= Duration::from_secs(OPEN_SKILLS_SYNC_INTERVAL_SECS)
}

/// 标记开放技能仓库已完成同步
///
/// 创建或更新同步标记文件，记录当前时间为最后同步时间。
/// 该文件的存在和修改时间用于判断是否需要进行新的同步。
///
/// # 参数
///
/// - `repo_dir`: 本地仓库路径
///
/// # 返回值
///
/// - `Ok(())`: 标记文件创建/更新成功
/// - `Err(...)`: 文件操作失败
fn mark_open_skills_synced(repo_dir: &Path) -> anyhow::Result<()> {
    std::fs::write(repo_dir.join(OPEN_SKILLS_SYNC_MARKER), b"synced")?;
    Ok(())
}

/// 从开放技能仓库加载所有技能定义
///
/// 扫描指定目录，加载所有符合要求的技能定义文件。
/// 支持两种目录结构：
/// 1. 嵌套结构：`skills/<name>/SKILL.md`（优先）
/// 2. 扁平结构：根目录下的 `.md` 文件（兼容旧版）
///
/// # 参数
///
/// - `repo_dir`: 开放技能仓库的根目录路径
/// - `load_mode`: 技能加载模式（确定加载哪些字段）
///
/// # 返回值
///
/// 返回成功加载的技能列表。如果目录不存在或读取失败，返回空列表。
///
/// # 安全审计
///
/// 每个技能文件在加载前都会进行安全审计：
/// - 检查文件是否包含潜在的不安全内容
/// - 跳过未通过审计的文件并记录警告
/// - 跳过无法审计的文件并记录警告
///
/// # 文件过滤规则
///
/// - 仅处理 `.md` 扩展名的文件
/// - 排除 `README.md` 文件（仓库文档而非技能定义）
/// - 排除所有未通过安全审计的文件
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::skills::types::SkillLoadMode;
///
/// let skills = load_open_skills(&repo_dir, SkillLoadMode::Full);
/// println!("已加载 {} 个开放技能", skills.len());
/// ```
pub(crate) fn load_open_skills(
    repo_dir: &Path,
    load_mode: SkillLoadMode,
    security: &vw_config_types::skills::SkillSecurityConfig,
) -> Vec<crate::app::agent::skills::types::Skill> {
    let nested_skills_dir = repo_dir.join("skills");
    if nested_skills_dir.is_dir() {
        return load_skills_from_directory(&nested_skills_dir, load_mode, security);
    }

    let mut skills = Vec::new();

    let Ok(entries) = std::fs::read_dir(repo_dir) else {
        return skills;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let is_markdown = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
        if !is_markdown {
            continue;
        }

        let is_readme = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("README.md"));
        if is_readme {
            continue;
        }

        match crate::app::agent::skills::audit::audit_open_skill_markdown_with_security(
            &path, repo_dir, security,
        ) {
            Ok(report) if report.is_clean() => {}
            Ok(report) => {
                tracing::warn!(
                    "skipping insecure open-skill file {}: {}",
                    path.display(),
                    report.summary()
                );
                continue;
            }
            Err(err) => {
                tracing::warn!("skipping unauditable open-skill file {}: {err}", path.display());
                continue;
            }
        }

        if let Ok(skill) = crate::app::agent::skills::loader::load_open_skill_md(&path, load_mode) {
            skills.push(skill);
        }
    }

    skills
}
#[cfg(test)]
#[path = "open_skills_tests.rs"]
mod open_skills_tests;
