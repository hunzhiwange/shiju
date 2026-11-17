//! # 技能模块（Skills Module）
//!
//! 本模块提供了代理技能的加载、初始化和管理功能。
//!
//! ## 模块职责
//!
//! - **技能加载**：从本地目录和插件加载技能定义
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
//! - `policy`：技能策略管理
//! - `prompt`：技能提示词生成
//! - `source`：技能源管理
//! - `types`：技能类型定义

use std::path::Path;

mod agent_metadata;
mod audit;
mod cli;
mod discovery;
mod init;
mod installer;
mod invoked;
mod listing;
mod loader;
pub(crate) mod marketplace;
mod policy;
mod prompt;
pub mod session_mounts;
mod session_state;
pub(crate) mod shiju;
mod source;
mod types;

pub use agent_metadata::{
    SkillAgentDependency, SkillAgentInterface, SkillAgentMetadata, SkillAgentPolicy,
};
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use audit::audit_open_skill_markdown_with_security;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) use invoked::InvokedSkillRecord;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use invoked::{InvokedSkillRecord, InvokedSkillReplay};
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use loader::{load_skill_directory, read_markdown_skill_metadata};
pub use prompt::{skills_to_prompt, skills_to_prompt_with_mode};
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use session_state::SkillListingSessionState;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use types::SkillLoadMode;
pub use types::{PluginSkillSource, Skill, SkillTool};

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use agent_skills::local::{is_local_skill_disabled, local_skill_disabled_marker_path};
#[cfg(all(not(test), not(target_arch = "wasm32")))]
pub(crate) use agent_skills::local_sources::discover_local_skill_source_dirs_with_provider;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use agent_skills::local_sources::{
    LocalSkillSourceKind, discover_local_skill_source_dirs_with_provider, workspace_skills_dir,
};

pub use agent_skills::catalog::{load_skills, skills_dir};

/// Compose local and active-tenant plugin inventories using runtime configuration.
pub fn load_skills_with_config(
    workspace: &Path,
    config: &crate::app::agent::config::Config,
) -> Vec<Skill> {
    agent_skills::catalog::load_skills_with_config(
        workspace,
        &config.skills,
        false,
        |mode| crate::plugins::load_plugin_skills(workspace, config, mode),
    )
}

pub(crate) fn load_skills_full_with_config(
    workspace: &Path,
    config: &crate::app::agent::config::Config,
) -> Vec<Skill> {
    agent_skills::catalog::load_skills_with_config(
        workspace,
        &config.skills,
        true,
        |mode| crate::plugins::load_plugin_skills(workspace, config, mode),
    )
}

pub(crate) fn load_project_skills_with_config(
    workspace: &Path,
    config: &crate::app::agent::config::Config,
) -> Vec<Skill> {
    agent_skills::catalog::load_project_skills_with_config(workspace, &config.skills)
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

/// Load the same inbound session sources for catalog refresh and native invocation.
pub(crate) fn load_session_skills(
    workspace: &Path,
    config: &crate::config::Config,
    sources: &crate::acp::mounts::SessionSkillSources,
    full: bool,
) -> anyhow::Result<Vec<Skill>> {
    agent_skills::catalog::load_session_skills(
        workspace,
        &config.skills,
        sources,
        full,
        |mode| crate::plugins::load_plugin_skills(workspace, config, mode),
    )
}

#[cfg(test)]
mod mod_tests;
