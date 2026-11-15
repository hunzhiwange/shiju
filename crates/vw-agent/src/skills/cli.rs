//! 技能模块命令行接口
//!
//! 本模块提供技能（Skill）相关的命令行处理功能，是用户与技能系统交互的主要入口。
//! 支持的命令包括：
//! - `list`: 列出所有已安装的技能
//! - `audit`: 审计指定技能的安全性
//! - `install`: 从指定来源安装技能
//! - `remove`: 移除已安装的技能
//!
//! # 架构说明
//!
//! 本模块采用条件编译策略，在 WASM 目标平台上提供存根实现，
//! 在原生平台上提供完整功能。所有命令处理函数都会执行必要的安全检查，
//! 确保技能操作的安全性和可靠性。
//!
//! # 安全性考虑
//!
//! - 路径遍历攻击防护：在移除技能时严格验证路径
//! - 权限边界检查：确保操作不会越出技能目录范围
//! - 审计集成：安装时自动执行安全审计

use crate::app::agent::skills::installer::{InstallResult, install_skill_from_source};
#[cfg(test)]
use crate::app::agent::skills::skills_dir;
use crate::app::agent::skills::{
    init_skills_dir, load_skills_full_with_config, workspace_skills_dir,
};
use anyhow::Result;

/// 处理技能相关的命令行命令（原生平台实现）
///
/// 该函数是技能命令的分发器，根据命令类型执行相应的操作。
/// 仅在非 WASM 目标平台上可用。
///
/// # 参数
///
/// * `command` - 技能命令枚举，指定要执行的操作类型
///   - `List`: 列出所有已安装的技能
///   - `Audit { source }`: 审计指定路径或名称的技能
///   - `Install { source }`: 从指定来源安装技能
///   - `Remove { name }`: 移除指定名称的技能
/// * `config` - 应用配置引用，提供工作空间目录等必要信息
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回包含错误信息的 `Err`
///
/// # 错误
///
/// 该函数可能在以下情况下返回错误：
/// - 技能源或已安装的技能不存在（Audit/Remove）
/// - 技能审计未通过（Audit/Install）
/// - 技能名称包含非法字符（Remove）
/// - 技能路径越界（Remove）
/// - 文件系统操作失败（Install/Remove）
///
/// # 示例
///
/// ```ignore
/// use crate::app::agent::skill::SkillCommands;
/// use crate::app::agent::config::Config;
///
/// let config = Config::load()?;
/// let command = SkillCommands::List;
/// handle_command(command, &config)?;
/// ```
///
/// # 安全性
///
/// - 在移除技能时，会严格检查路径遍历攻击
/// - 确保操作路径始终位于技能目录范围内
/// - 安装操作会自动执行安全审计
#[allow(clippy::too_many_lines)]
#[cfg(not(target_arch = "wasm32"))]
pub fn handle_command(
    command: crate::app::agent::skill::SkillCommands,
    config: &crate::app::agent::config::Config,
) -> Result<()> {
    // 获取工作空间目录，作为所有技能操作的基准路径
    let workspace_dir = &config.workspace_dir;

    match command {
        // 处理 list 命令：列出所有已安装的技能
        crate::app::agent::skill::SkillCommands::List => {
            // 加载所有技能，包括开放技能（如果启用）
            let skills = load_skills_full_with_config(workspace_dir, config);

            let entries = skills.iter().map(|skill| agent_skills::cli::SkillListEntry {
                name: &skill.name,
                version: &skill.version,
                description: &skill.description,
                tools: skill.tools.iter().map(|tool| tool.name.as_str()).collect(),
                tags: &skill.tags,
            }).collect::<Vec<_>>();
            agent_skills::cli::print_skill_list(workspace_dir, &entries)
        }

        // 处理 audit 命令：审计指定技能的安全性
        crate::app::agent::skill::SkillCommands::Audit { source } => {
            agent_skills::cli::audit_skill(
                source,
                &workspace_skills_dir(workspace_dir, config.skills.directory_provider),
                &config.skills.security,
            )
        }

        // 处理 install 命令：从指定来源安装技能
        crate::app::agent::skill::SkillCommands::Install { source } => {
            let skills_path = workspace_skills_dir(workspace_dir, config.skills.directory_provider);
            agent_skills::cli::install_skill(
                &source,
                &skills_path,
                || init_skills_dir(workspace_dir),
                |source, path| {
                    let result = install_skill_from_source(source, path, &config.skills.security)?;
                    print_install_success(result);
                    Ok(())
                },
            )
        }

        // 处理 remove 命令：移除已安装的技能
        crate::app::agent::skill::SkillCommands::Remove { name } => {
            agent_skills::cli::remove_skill(
                name,
                &workspace_skills_dir(workspace_dir, config.skills.directory_provider),
            )
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn print_install_success(install_result: InstallResult) {
    let (message, installed_dir, files_scanned) = match install_result {
        InstallResult::SkillsSh { installed_dir, files_scanned } =>
            ("Skill installed from skills.sh", installed_dir, files_scanned),
        InstallResult::Git { installed_dir, files_scanned } =>
            ("Skill installed and audited", installed_dir, files_scanned),
        InstallResult::Local { installed_dir, files_scanned } =>
            ("Skill installed and audited", installed_dir, files_scanned),
    };
    agent_skills::cli::print_install_success(message, &installed_dir, files_scanned);
}

/// 处理技能相关的命令行命令（WASM 平台存根实现）
///
/// 该函数是技能命令处理在 WASM 目标平台上的存根实现。
/// 由于 WASM 环境不支持文件系统操作，技能功能在此平台上不可用。
///
/// # 参数
///
/// * `_command` - 技能命令枚举（未使用，因为功能不可用）
/// * `_config` - 应用配置引用（未使用，因为功能不可用）
///
/// # 返回值
///
/// 始终返回错误，提示技能命令在 WASM 平台上不支持
///
/// # 错误
///
/// 总是返回错误信息："Skill commands are not supported on WASM"
#[cfg(target_arch = "wasm32")]
pub fn handle_command(
    _command: crate::app::agent::skill::SkillCommands,
    _config: &crate::app::agent::config::Config,
) -> Result<()> {
    anyhow::bail!("Skill commands are not supported on WASM")
}
#[cfg(test)]
#[path = "cli_tests.rs"]
mod cli_tests;
