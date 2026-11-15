//! 技能目录初始化模块
//!
//! 本模块提供技能目录的初始化功能，负责在指定工作空间中创建标准的技能目录结构，
//! 并生成必要的说明文档和配置文件。
//!
//! # 主要功能
//!
//! - 创建技能目录结构
//! - 生成 README.md 说明文档
//! - 确保内置预加载技能已就绪
//! - 初始化技能下载策略配置

use crate::app::agent::skills::installer::ensure_builtin_preloaded_skills;
use crate::app::agent::skills::policy::load_or_init_skill_download_policy;
use crate::app::agent::skills::skills_dir;
use anyhow::Result;
use std::path::Path;

/// 初始化技能目录
///
/// 在指定的工作空间目录下创建技能目录结构，并生成必要的说明文档。
/// 如果目录或文件已存在，则不会覆盖。
///
/// # 参数
///
/// - `workspace_dir`: 工作空间根目录路径，技能目录将在此目录下创建
///
/// # 返回值
///
/// 返回 `Result<()>`，成功时为 `Ok(())`，失败时返回错误信息
///
/// # 错误
///
/// 在以下情况可能返回错误：
/// - 无法创建目录结构
/// - 无法写入 README.md 文件
/// - 内置技能预加载失败
///
/// # 示例
///
/// ```no_run
/// use std::path::Path;
/// use vibewindow::app::agent::skills::init::init_skills_dir;
///
/// let workspace = Path::new("/path/to/workspace");
/// init_skills_dir(workspace)?;
/// ```
pub fn init_skills_dir(workspace_dir: &Path) -> Result<()> {
    let dir = skills_dir(workspace_dir);
    agent_skills::init::initialize_skills_dir(
        &dir,
        ensure_builtin_preloaded_skills,
        |path| load_or_init_skill_download_policy(path).map(|_| ()),
    )
}
#[cfg(test)]
#[path = "init_tests.rs"]
mod init_tests;
