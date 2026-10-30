//! 技能系统类型定义模块
//!
//! 本模块定义了 VibeWindow 技能系统的核心数据类型，包括：
//! - 技能元数据与清单结构
//! - 技能下载策略配置
//! - 技能工具定义
//! - 技能加载模式枚举
//!
//! 技能是用户自定义或社区构建的能力扩展，存储在 `~/.shiju-agent/workspace/skills/<name>/SKILL.md`，
//! 可以包含工具定义、提示词模板和自动化脚本。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::path::PathBuf;

/// 默认预加载技能源列表
///
/// 定义系统启动时自动预加载的技能别名与源标识映射。
/// 目前包含三个核心技能：
/// - `create-task-plans`: 创建实施计划
/// - `do-task-plans`: 执行实施计划
/// - `rustcodegraph`: Rust 代码图导航
const DEFAULT_PRELOADED_SKILL_SOURCES: [(&str, &str); 3] = [
    ("create-task-plans", "bundled:create-task-plans"),
    ("do-task-plans", "bundled:do-task-plans"),
    ("rustcodegraph", "bundled:rustcodegraph"),
];

/// 获取默认预加载技能别名字典
///
/// 将常量数组转换为可序列化的 BTreeMap，用于配置文件序列化。
///
/// # 返回值
///
/// 返回技能别名到源标识的有序映射字典
///
/// # 示例
///
/// ```
/// let aliases = default_preloaded_skill_aliases();
/// assert!(aliases.contains_key("create-task-plans"));
/// assert!(aliases.contains_key("do-task-plans"));
/// assert!(aliases.contains_key("rustcodegraph"));
/// ```
pub fn default_preloaded_skill_aliases() -> BTreeMap<String, String> {
    DEFAULT_PRELOADED_SKILL_SOURCES
        .iter()
        .map(|(alias, source)| ((*alias).to_string(), (*source).to_string()))
        .collect()
}

/// 获取默认策略版本号
///
/// 返回技能下载策略的当前默认版本，用于配置迁移和兼容性检查。
///
/// # 返回值
///
/// 返回默认策略版本号（当前为 1）
pub fn default_policy_version() -> u32 {
    1
}

/// 技能下载策略配置
///
/// 定义技能下载的安全策略，包括别名映射、信任域名和阻止域名。
/// 该配置用于控制技能源的访问权限和风险隔离。
///
/// # 字段说明
///
/// - `version`: 策略版本号，用于未来配置迁移
/// - `aliases`: 技能别名到源 URL 的映射，便于快速引用常用技能
/// - `trusted_domains`: 受信任的域名列表，从此类域名下载技能时放宽限制
/// - `blocked_domains`: 阻止的域名列表，禁止从此类域名下载任何技能
///
/// # 示例
///
/// ```
/// let policy = SkillDownloadPolicy::default();
/// assert_eq!(policy.version, 1);
/// assert!(policy.aliases.contains_key("create-task-plans"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDownloadPolicy {
    /// 策略版本号
    #[serde(default = "default_policy_version")]
    pub version: u32,

    /// 技能别名到源 URL 的映射字典
    #[serde(default = "default_preloaded_skill_aliases")]
    pub aliases: BTreeMap<String, String>,

    /// 受信任的域名白名单
    #[serde(default)]
    pub trusted_domains: Vec<String>,

    /// 阻止的域名黑名单
    #[serde(default)]
    pub blocked_domains: Vec<String>,
}

impl Default for SkillDownloadPolicy {
    /// 创建默认的技能下载策略
    ///
    /// 使用默认版本号、预加载技能别名和空域名列表初始化策略。
    fn default() -> Self {
        Self {
            version: default_policy_version(),
            aliases: default_preloaded_skill_aliases(),
            trusted_domains: Vec::new(),
            blocked_domains: Vec::new(),
        }
    }
}

/// 技能定义结构
///
/// 技能是用户定义或社区构建的能力单元，存储在 `~/.shiju-agent/workspace/skills/<name>/SKILL.md`。
/// 每个技能可以包含工具定义、提示词模板和自动化脚本。
///
/// # 字段说明
///
/// - `name`: 技能唯一标识名称
/// - `description`: 技能功能描述
/// - `version`: 语义化版本号
/// - `author`: 可选的作者信息
/// - `tags`: 分类标签列表，便于搜索和过滤
/// - `tools`: 技能提供的工具列表
/// - `prompts`: 技能提供的提示词模板列表
/// - `location`: 技能文件在本地文件系统的路径（运行时填充，不参与序列化）
///
/// # 示例
///
/// ```
/// let skill = Skill {
///     name: "code-review".to_string(),
///     description: "代码审查助手".to_string(),
///     version: "1.0.0".to_string(),
///     author: Some("VibeWindow Team".to_string()),
///     tags: vec!["code".to_string(), "review".to_string()],
///     tools: vec![],
///     prompts: vec![],
///     location: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// 技能名称
    pub name: String,

    /// 技能描述
    pub description: String,

    /// 插件技能的可选单行短说明。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_description: Option<String>,

    /// 运行时审计产生的插件来源，不能由技能文件声明或反序列化伪造。
    #[serde(skip)]
    pub plugin_source: Option<PluginSkillSource>,

    /// Audited agents/openai.yaml metadata; files cannot forge this runtime projection.
    #[serde(default, skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub agent_metadata: Option<super::SkillAgentMetadata>,

    /// 技能版本号
    pub version: String,

    /// 技能作者
    #[serde(default)]
    pub author: Option<String>,

    /// 分类标签
    #[serde(default)]
    pub tags: Vec<String>,

    /// 技能提供的工具列表
    #[serde(default)]
    pub tools: Vec<SkillTool>,

    /// 技能提供的提示词列表
    #[serde(default)]
    pub prompts: Vec<String>,

    /// 技能文件在本地文件系统的路径（运行时设置，不参与序列化）
    #[serde(skip)]
    pub location: Option<PathBuf>,

    /// 描述模型应在何时考虑调用该技能。
    #[serde(default)]
    pub when_to_use: Option<String>,

    /// 技能调用期间允许使用的工具名称。
    #[serde(default)]
    pub allowed_tools: Vec<String>,

    /// 技能请求使用的模型覆盖。
    #[serde(default)]
    pub model: Option<String>,

    /// 是否禁止模型自动调用，只允许用户显式调用。
    #[serde(default)]
    pub disable_model_invocation: bool,

    /// 是否允许用户通过技能选择器显式调用。
    #[serde(default = "default_user_invocable")]
    pub user_invocable: bool,

    /// 技能生命周期钩子定义，保留原始结构供调用层处理。
    #[serde(default)]
    pub hooks: Option<serde_json::Value>,

    /// 调用后应采用的上下文策略。
    #[serde(default)]
    pub context: Option<String>,

    /// 仅对匹配这些相对路径的文件生效；空列表表示无路径限制。
    #[serde(default)]
    pub paths: Vec<String>,

    /// 技能请求的推理强度覆盖。
    #[serde(default)]
    pub effort: Option<String>,

    /// 用户显式调用技能时展示的参数提示。
    #[serde(default)]
    pub argument_hint: Option<String>,
}

/// 技能所属插件及原包内资源定位；相对资源以技能文件所在目录为基准。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginSkillSource {
    pub plugin_id: String,
    pub namespace: String,
    pub root: PathBuf,
    pub relative_skill_path: PathBuf,
}

impl Skill {
    /// 返回技能是否适用于给定的工作区相对路径。
    pub fn applies_to_path(&self, path: &Path) -> bool {
        self.paths.is_empty()
            || self.paths.iter().any(|pattern| {
                glob::Pattern::new(pattern).is_ok_and(|pattern| pattern.matches_path(path))
            })
    }
}

impl Default for Skill {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            short_description: None,
            plugin_source: None,
            agent_metadata: None,
            version: default_version(),
            author: None,
            tags: Vec::new(),
            tools: Vec::new(),
            prompts: Vec::new(),
            location: None,
            when_to_use: None,
            allowed_tools: Vec::new(),
            model: None,
            disable_model_invocation: false,
            user_invocable: default_user_invocable(),
            hooks: None,
            context: None,
            paths: Vec::new(),
            effort: None,
            argument_hint: None,
        }
    }
}

fn default_user_invocable() -> bool {
    true
}

/// 技能工具定义
///
/// 定义技能提供的一个可执行工具，可以是 shell 命令、HTTP 请求或脚本。
///
/// # 字段说明
///
/// - `name`: 工具名称，用于调用时标识
/// - `description`: 工具功能描述
/// - `kind`: 工具类型（"shell"、"http"、"script"）
/// - `command`: 要执行的命令、URL 或脚本内容
/// - `args`: 工具参数映射，键为参数名，值为参数描述或默认值
///
/// # 示例
///
/// ```
/// let tool = SkillTool {
///     name: "lint".to_string(),
///     description: "运行代码检查".to_string(),
///     kind: "shell".to_string(),
///     command: "cargo clippy".to_string(),
///     args: HashMap::new(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTool {
    /// 工具名称
    pub name: String,

    /// 工具描述
    pub description: String,

    /// 工具类型（"shell"、"http"、"script"）
    pub kind: String,

    /// 要执行的命令、URL 或脚本内容
    pub command: String,

    /// 工具参数映射
    #[serde(default)]
    pub args: HashMap<String, String>,
}

/// 技能清单结构
///
/// 从 SKILL.toml 文件解析的完整技能清单，包含技能元数据、工具和提示词。
/// 该结构仅用于内部解析，不对外暴露。
///
/// # 字段说明
///
/// - `skill`: 技能元数据
/// - `tools`: 技能提供的工具列表
/// - `prompts`: 技能提供的提示词列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillManifest {
    /// 技能元数据
    pub skill: SkillMeta,

    /// 技能工具列表
    #[serde(default)]
    pub tools: Vec<SkillTool>,

    /// 技能提示词列表
    #[serde(default)]
    pub prompts: Vec<String>,
}

/// 技能元数据清单结构
///
/// 仅包含技能元数据的精简清单结构，用于快速加载技能基本信息。
/// 该结构仅用于内部解析，不对外暴露。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillMetadataManifest {
    /// 技能元数据
    pub skill: SkillMeta,
}

/// 技能元数据结构
///
/// 定义技能的核心元数据信息，包括名称、描述、版本、作者和标签。
/// 该结构用于清单文件解析和元数据交换。
///
/// # 字段说明
///
/// - `name`: 技能唯一标识名称
/// - `description`: 技能功能描述
/// - `version`: 语义化版本号（默认 "0.1.0"）
/// - `author`: 可选的作者信息
/// - `tags`: 分类标签列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillMeta {
    /// 技能名称
    pub name: String,

    /// 技能描述
    pub description: String,

    /// 技能版本号
    #[serde(default = "default_version")]
    pub version: String,

    /// 技能作者
    #[serde(default)]
    pub author: Option<String>,

    /// 分类标签
    #[serde(default)]
    pub tags: Vec<String>,
}

/// 获取默认技能版本号
///
/// 返回技能的默认版本号 "0.1.0"，用于未指定版本的新技能。
///
/// # 返回值
///
/// 返回默认版本字符串 "0.1.0"
pub(crate) fn default_version() -> String {
    "0.1.0".to_string()
}

/// 技能加载模式枚举
///
/// 定义技能加载的两种模式：
/// - `Full`: 完整加载，包含所有工具和提示词定义
/// - `MetadataOnly`: 仅加载元数据，用于轻量级场景
///
/// 该枚举用于控制技能加载的粒度，平衡性能与功能完整性。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillLoadMode {
    /// 完整模式：加载技能的全部内容
    Full,

    /// 元数据模式：仅加载技能的基本信息
    MetadataOnly,
}

impl SkillLoadMode {
    /// 从配置的提示词注入模式转换为加载模式
    ///
    /// 将用户配置的 `SkillsPromptInjectionMode` 映射为内部的 `SkillLoadMode`。
    ///
    /// # 参数
    ///
    /// - `mode`: 配置中的提示词注入模式
    ///
    /// # 返回值
    ///
    /// 返回对应的技能加载模式：
    /// - `Full` 注入模式 -> `Full` 加载模式
    /// - `Compact` 注入模式 -> `MetadataOnly` 加载模式
    ///
    /// # 示例
    ///
    /// ```
    /// use crate::app::agent::config::SkillsPromptInjectionMode;
    /// let mode = SkillLoadMode::from_prompt_mode(SkillsPromptInjectionMode::Full);
    /// assert_eq!(mode, SkillLoadMode::Full);
    /// ```
    pub(crate) fn from_prompt_mode(
        mode: crate::app::agent::config::SkillsPromptInjectionMode,
    ) -> Self {
        match mode {
            crate::app::agent::config::SkillsPromptInjectionMode::Full => Self::Full,
            crate::app::agent::config::SkillsPromptInjectionMode::Compact => Self::MetadataOnly,
        }
    }
}
#[cfg(test)]
#[path = "types_tests.rs"]
mod types_tests;
