//! 技能系统类型定义模块
//!
//! 本模块定义了 VibeWindow 技能系统的核心数据类型和结构，包括：
//! - 技能元数据（Skill、SkillMeta、SkillManifest）
//! - 技能工具定义（SkillTool）
//! - 技能下载策略（SkillDownloadPolicy）
//! - 技能配置（SkillsConfig、SkillRuntimeConfig）
//! - 技能命令（SkillCommands）
//! - 技能加载模式（SkillLoadMode）
//!
//! 技能是用户定义或社区构建的能力扩展，位于 `~/.vibewindow/workspace/skills/<name>/SKILL.md`，
//! 可以包含工具定义、提示词模板和自动化脚本。

use crate::app::agent::skill::constants::{
    default_policy_version, default_preloaded_skill_aliases, default_version,
};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// 技能下载策略配置
///
/// 定义技能从外部源下载时的安全策略，包括版本控制、域名信任和别名映射。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillDownloadPolicy {
    /// 策略版本号，用于未来策略格式升级时的兼容性检查
    #[serde(default = "default_policy_version")]
    pub(crate) version: u32,

    /// 技能别名映射表，将简短别名映射到完整技能名称
    /// 例如：{"web": "web-scraping"} 可以用 `vw skill install web` 安装 web-scraping 技能
    #[serde(default = "default_preloaded_skill_aliases")]
    pub(crate) aliases: BTreeMap<String, String>,

    /// 受信任域名列表，来自这些域名的技能可自动安装而无需确认
    #[serde(default)]
    pub(crate) trusted_domains: Vec<String>,

    /// 禁止域名列表，来自这些域名的技能将被拒绝安装
    #[serde(default)]
    pub(crate) blocked_domains: Vec<String>,
}

impl Default for SkillDownloadPolicy {
    /// 创建默认的技能下载策略
    ///
    /// 默认策略包含预置的技能别名映射，但不包含任何信任或禁止域名。
    fn default() -> Self {
        Self {
            version: default_policy_version(),
            aliases: default_preloaded_skill_aliases(),
            trusted_domains: Vec::new(),
            blocked_domains: Vec::new(),
        }
    }
}

/// skills.sh 源标识符
///
/// 表示从 skills.sh 仓库获取技能时的源信息，包含 GitHub 仓库的所有者、
/// 仓库名和技能名称。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillsShSource {
    /// GitHub 仓库所有者
    pub(crate) owner: String,

    /// GitHub 仓库名称
    pub(crate) repo: String,

    /// 技能名称
    pub(crate) skill: String,
}

impl SkillsShSource {
    /// 生成 GitHub 仓库的 Git 克隆 URL
    ///
    /// 返回格式：`https://github.com/{owner}/{repo}.git`
    pub(crate) fn github_repo_url(&self) -> String {
        format!("https://github.com/{}/{}.git", self.owner, self.repo)
    }
}

/// 技能定义
///
/// 技能是用户定义或社区构建的能力扩展。每个技能存储在独立目录中，
/// 路径为 `~/.vibewindow/workspace/skills/<name>/SKILL.md`，可以包含：
/// - 工具定义（shell 命令、HTTP 调用等）
/// - 提示词模板
/// - 自动化脚本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// 技能名称，用于在命令行和配置中引用该技能
    pub name: String,

    /// 技能描述，简要说明该技能的功能和用途
    pub description: String,

    /// 技能版本号，遵循语义化版本规范（例如："1.0.0"）
    pub version: String,

    /// 技能作者信息（可选）
    #[serde(default)]
    pub author: Option<String>,

    /// 技能标签列表，用于分类和搜索
    #[serde(default)]
    pub tags: Vec<String>,

    /// 技能定义的工具列表
    #[serde(default)]
    pub tools: Vec<SkillTool>,

    /// 技能包含的提示词模板列表
    #[serde(default)]
    pub prompts: Vec<String>,

    /// 技能文件系统位置（不序列化，仅在运行时使用）
    #[serde(skip)]
    pub location: Option<PathBuf>,
}

/// 技能工具定义
///
/// 定义由技能提供的可执行工具，可以是 shell 命令、HTTP 请求或自定义脚本。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    /// 工具名称，用于在技能中引用该工具
    pub name: String,

    /// 工具描述，说明工具的功能和用途
    pub description: String,

    /// 工具类型，支持以下几种：
    /// - "shell": 执行 shell 命令
    /// - "http": 发送 HTTP 请求
    /// - "script": 执行自定义脚本
    pub kind: String,

    /// 要执行的命令、URL 或脚本内容
    pub command: String,

    /// 工具参数映射，定义参数名称和默认值的键值对
    #[serde(default)]
    pub args: HashMap<String, String>,
}

/// 技能清单（从 SKILL.toml 解析）
///
/// 包含技能的完整元数据、工具定义和提示词模板。
/// 这是从技能目录中的 SKILL.toml 文件解析得到的完整清单。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillManifest {
    /// 技能元数据
    pub(crate) skill: SkillMeta,

    /// 技能定义的工具列表
    #[serde(default)]
    pub(crate) tools: Vec<SkillTool>,

    /// 技能包含的提示词模板列表
    #[serde(default)]
    pub(crate) prompts: Vec<String>,
}

/// 技能元数据清单（仅包含元数据）
///
/// 用于仅需要技能基本信息（不包括工具和提示词）的场景，
/// 如技能列表展示或快速元数据扫描。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillMetadataManifest {
    /// 技能元数据
    pub(crate) skill: SkillMeta,
}

/// 技能元数据
///
/// 包含技能的基本信息，如名称、描述、版本、作者和标签。
/// 这是技能清单中的核心元数据部分。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillMeta {
    /// 技能名称
    pub(crate) name: String,

    /// 技能描述
    pub(crate) description: String,

    /// 技能版本号
    #[serde(default = "default_version")]
    pub(crate) version: String,

    /// 技能作者（可选）
    #[serde(default)]
    pub(crate) author: Option<String>,

    /// 技能标签列表
    #[serde(default)]
    pub(crate) tags: Vec<String>,
}

/// 技能提示词注入模式
///
/// 定义如何将技能信息注入到代理的提示词上下文中。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillsPromptInjectionMode {
    /// 完整模式：注入完整的技能定义，包括工具和提示词
    Full,

    /// 紧凑模式：仅注入技能元数据，减少上下文长度
    Compact,
}

/// 技能系统配置
///
/// 定义技能系统的全局配置选项。
#[derive(Debug, Clone)]
pub struct SkillsConfig {
    /// 提示词注入模式，控制如何将技能信息注入到代理上下文
    pub prompt_injection_mode: SkillsPromptInjectionMode,
}

impl Default for SkillsConfig {
    /// 创建默认的技能配置
    ///
    /// 默认配置：
    /// - 使用完整提示词注入模式
    fn default() -> Self {
        Self { prompt_injection_mode: SkillsPromptInjectionMode::Full }
    }
}

/// 技能运行时配置
///
/// 包含技能系统运行时所需的配置信息，结合工作区路径和技能配置。
#[derive(Debug, Clone)]
pub struct SkillRuntimeConfig {
    /// 工作区根目录路径，技能将在此目录下的 skills 子目录中管理
    pub workspace_dir: PathBuf,

    /// 技能系统配置
    pub skills: SkillsConfig,
}

impl Default for SkillRuntimeConfig {
    /// 创建默认的技能运行时配置
    ///
    /// 默认配置使用当前目录作为工作区
    fn default() -> Self {
        Self { workspace_dir: PathBuf::from("."), skills: SkillsConfig::default() }
    }
}

/// 技能管理命令枚举
///
/// 定义通过 CLI 可执行的技能管理操作。
#[derive(Debug, Clone, Subcommand)]
pub enum SkillCommands {
    /// 列出所有已安装的技能
    List,

    /// 审计指定源的技能（检查安全性、合规性等）
    Audit { source: String },

    /// 从指定源安装技能
    Install { source: String },

    /// 移除指定名称的技能
    Remove { name: String },
}

/// 技能加载模式
///
/// 定义加载技能时的详细程度，影响哪些数据会被从磁盘加载到内存。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillLoadMode {
    /// 完整加载：加载技能的所有信息（元数据 + 工具 + 提示词）
    Full,

    /// 仅元数据：仅加载技能的基本信息，不包括工具和提示词
    MetadataOnly,
}

impl SkillLoadMode {
    /// 从提示词注入模式创建加载模式
    ///
    /// 根据提示词注入模式决定加载策略：
    /// - Full 模式 -> 完整加载
    /// - Compact 模式 -> 仅加载元数据
    pub(crate) fn from_prompt_mode(mode: SkillsPromptInjectionMode) -> Self {
        match mode {
            SkillsPromptInjectionMode::Full => Self::Full,
            SkillsPromptInjectionMode::Compact => Self::MetadataOnly,
        }
    }
}
#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
