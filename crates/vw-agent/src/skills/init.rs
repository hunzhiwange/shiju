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
    // 获取技能目录路径
    let dir = skills_dir(workspace_dir);
    // 创建技能目录及其所有父目录（如果不存在）
    std::fs::create_dir_all(&dir)?;

    // 构建 README.md 文件路径
    let readme = dir.join("README.md");
    // 仅在 README.md 不存在时创建，避免覆盖用户已有的自定义内容
    if !readme.exists() {
        // 写入技能目录的说明文档，包含 SKILL.toml 和 SKILL.md 格式说明
        std::fs::write(
            &readme,
            "# VibeWindow Skills\n\n\
             Each subdirectory is a skill. Create a `SKILL.toml` or `SKILL.md` file inside.\n\n\
             ## SKILL.toml format\n\n\
             ```toml\n\
             [skill]\n\
             name = \"my-skill\"\n\
             description = \"What this skill does\"\n\
             version = \"0.1.0\"\n\
             author = \"your-name\"\n\
             tags = [\"productivity\", \"automation\"]\n\n\
             [[tools]]\n\
             name = \"my_tool\"\n\
             description = \"What this tool does\"\n\
             kind = \"shell\"\n\
             command = \"echo hello\"\n\
             ```\n\n\
             ## SKILL.md format (simpler)\n\n\
             Just write a markdown file with instructions for the agent.\n\
             The agent will read it and follow the instructions.\n\n\
             ## Installing community skills\n\n\
             ```bash\n\
             shiju skills install <source>\n\
             shiju skills list\n\
             ```\n",
        )?;
    }

    // 确保内置预加载技能已安装到技能目录中
    ensure_builtin_preloaded_skills(&dir)?;
    // 加载或初始化技能下载策略配置，忽略错误以避免阻塞初始化流程
    let _ = load_or_init_skill_download_policy(&dir)?;

    Ok(())
}
#[cfg(test)]
#[path = "init_tests.rs"]
mod init_tests;
