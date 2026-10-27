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
use std::path::PathBuf;

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

            // 如果没有安装任何技能，显示帮助信息
            if skills.is_empty() {
                println!("No skills installed.");
                println!();
                println!(
                    "  Create one: mkdir -p {}",
                    workspace_dir.join("skills").join("my-skill").display()
                );
                println!(
                    "              echo '# My Skill' > {}",
                    workspace_dir.join("skills").join("my-skill").join("SKILL.md").display()
                );
                println!();
                println!("  Or install: shiju skills install <source>");
            } else {
                // 显示已安装技能的统计信息
                println!("Installed skills ({}):", skills.len());
                println!();

                // 遍历并显示每个技能的详细信息
                for skill in &skills {
                    // 显示技能名称、版本和描述
                    println!(
                        "  {} {} — {}",
                        console::style(&skill.name).white().bold(),
                        console::style(format!("v{}", skill.version)).dim(),
                        skill.description
                    );

                    // 如果技能包含工具，显示工具列表
                    if !skill.tools.is_empty() {
                        println!(
                            "    Tools: {}",
                            skill
                                .tools
                                .iter()
                                .map(|t| t.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }

                    // 如果技能有标签，显示标签列表
                    if !skill.tags.is_empty() {
                        println!("    Tags:  {}", skill.tags.join(", "));
                    }
                }
            }
            println!();
            Ok(())
        }

        // 处理 audit 命令：审计指定技能的安全性
        crate::app::agent::skill::SkillCommands::Audit { source } => {
            // 解析源路径：可能是绝对路径或技能名称
            let source_path = PathBuf::from(&source);

            // 确定审计目标：优先使用实际路径，否则在技能目录中查找
            let target = if source_path.exists() {
                source_path
            } else {
                workspace_skills_dir(workspace_dir, config.skills.directory_provider).join(&source)
            };

            // 验证目标是否存在
            if !target.exists() {
                anyhow::bail!("Skill source or installed skill not found: {source}");
            }

            // 执行技能安全审计
            let report = crate::app::agent::skills::audit::audit_skill_directory_with_security(
                &target,
                &config.skills.security,
            )?;

            // 如果审计通过（无安全问题），显示成功信息
            if report.is_clean() {
                println!(
                    "  {} Skill audit passed for {} ({} files scanned).",
                    console::style("✓").green().bold(),
                    target.display(),
                    report.files_scanned
                );
                return Ok(());
            }

            // 审计未通过，显示所有发现的问题
            println!(
                "  {} Skill audit failed for {}",
                console::style("✗").red().bold(),
                target.display()
            );
            for finding in report.findings {
                println!("    - {finding}");
            }
            anyhow::bail!("Skill audit failed.");
        }

        // 处理 install 命令：从指定来源安装技能
        crate::app::agent::skill::SkillCommands::Install { source } => {
            println!("Installing skill from: {source}");

            // 初始化技能目录（如果不存在则创建）
            init_skills_dir(workspace_dir)?;
            let skills_path = workspace_skills_dir(workspace_dir, config.skills.directory_provider);
            std::fs::create_dir_all(&skills_path).map_err(|err| {
                anyhow::anyhow!("failed to create {}: {err}", skills_path.display())
            })?;

            // 执行技能安装，支持多种来源（本地、Git、skills.sh）
            let install_result =
                install_skill_from_source(&source, &skills_path, &config.skills.security)?;

            print_install_success(install_result);

            println!("  Security audit completed successfully.");
            Ok(())
        }

        // 处理 remove 命令：移除已安装的技能
        crate::app::agent::skill::SkillCommands::Remove { name } => {
            // 安全校验：拒绝路径遍历攻击尝试
            // 禁止在技能名称中包含父目录引用或路径分隔符
            if name.contains("..") || name.contains('/') || name.contains('\\') {
                anyhow::bail!("Invalid skill name: {name}");
            }

            // 构建技能的完整路径
            let skills_path = workspace_skills_dir(workspace_dir, config.skills.directory_provider);
            let skill_path = skills_path.join(&name);

            // 安全校验：确保解析后的路径实际位于技能目录内部
            // 通过规范化路径来检测潜在的路径逃逸攻击
            let canonical_skills =
                skills_path.canonicalize().unwrap_or_else(|_| skills_path.clone());
            if let Ok(canonical_skill) = skill_path.canonicalize() {
                // 验证技能路径是否以技能目录为前缀
                if !canonical_skill.starts_with(&canonical_skills) {
                    anyhow::bail!("Skill path escapes skills directory: {name}");
                }
            }

            // 验证技能是否存在
            if !skill_path.exists() {
                anyhow::bail!("Skill not found: {name}");
            }

            // 递归删除技能目录及其所有内容
            std::fs::remove_dir_all(&skill_path)?;
            println!("  {} Skill '{}' removed.", console::style("✓").green().bold(), name);
            Ok(())
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn print_install_success(install_result: InstallResult) {
    // 根据安装来源类型显示相应的成功信息
    match install_result {
        // 从 skills.sh 仓库安装
        InstallResult::SkillsSh { installed_dir, files_scanned } => {
            println!(
                "  {} Skill installed from skills.sh: {} ({} files scanned)",
                console::style("✓").green().bold(),
                installed_dir.display(),
                files_scanned
            );
        }
        // 从 Git 仓库安装
        InstallResult::Git { installed_dir, files_scanned } => {
            println!(
                "  {} Skill installed and audited: {} ({} files scanned)",
                console::style("✓").green().bold(),
                installed_dir.display(),
                files_scanned
            );
        }
        // 从本地目录安装
        InstallResult::Local { installed_dir, files_scanned } => {
            println!(
                "  {} Skill installed and audited: {} ({} files scanned)",
                console::style("✓").green().bold(),
                installed_dir.display(),
                files_scanned
            );
        }
    }
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
