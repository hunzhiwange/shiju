//! 技能模块常量定义
//!
//! 本模块定义了技能（Skill）子系统所需的所有常量配置，包括：
//! - 预加载技能的默认源列表
//! - 内置预加载技能的结构定义
//! - 下载策略文件的名称
//!
//! 这些常量用于技能的下载、同步、预加载等核心功能。

use std::collections::BTreeMap;

/// 技能下载策略配置文件名
///
/// 此文件用于控制技能的下载行为和权限设置。
pub(crate) const SKILL_DOWNLOAD_POLICY_FILE: &str = ".download-policy.toml";

/// skills.sh 服务主机名
///
/// 这是技能托管服务的基础域名，用于构建技能的远程访问 URL。
pub(crate) const SKILLS_SH_HOST: &str = "skills.sh";

/// 默认预加载技能源列表
///
/// 定义了默认需要预加载的技能别名与其对应的源标识。
/// 每个元组包含 (别名, 源标识)。
///
/// # 示例
///
/// ```
/// // 包含三个默认技能：
/// // - "create-task-plans": 创建实施计划
/// // - "do-task-plans": 执行实施计划
/// // - "rustcodegraph": Rust 代码图导航
/// ```
pub(crate) const DEFAULT_PRELOADED_SKILL_SOURCES: [(&str, &str); 3] = [
    ("create-task-plans", "bundled:create-task-plans"),
    ("do-task-plans", "bundled:do-task-plans"),
    ("rustcodegraph", "bundled:rustcodegraph"),
];

/// 内置预加载技能定义结构体
///
/// 描述一个内置技能的基本信息，包括目录名、源 URL 和技能的 Markdown 文档内容。
/// 这些信息用于在运行时初始化内置技能。
pub(crate) struct BuiltinPreloadedSkill {
    /// 技能在本地文件系统中的目录名称
    pub(crate) dir_name: &'static str,
    /// 技能的远程源 URL，用于更新和追溯
    pub(crate) source_url: &'static str,
    /// 技能的 Markdown 格式文档内容，在编译时通过 `include_str!` 宏嵌入
    pub(crate) markdown: &'static str,
}

/// 内置预加载技能数组
///
/// 包含所有需要内置预加载的技能定义。这些技能会在系统启动时自动加载，
/// 无需用户手动配置。
///
/// # 包含的技能
///
/// - `create-task-plans`: 创建实施计划
/// - `do-task-plans`: 执行实施计划
/// - `rustcodegraph`: Rust 代码图导航
pub(crate) const BUILTIN_PRELOADED_SKILLS: [BuiltinPreloadedSkill; 3] = [
    BuiltinPreloadedSkill {
        dir_name: "create-task-plans",
        source_url: "bundled:create-task-plans",
        markdown: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/skills/create-task-plans/SKILL.md"
        )),
    },
    BuiltinPreloadedSkill {
        dir_name: "do-task-plans",
        source_url: "bundled:do-task-plans",
        markdown: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/skills/do-task-plans/SKILL.md"
        )),
    },
    BuiltinPreloadedSkill {
        dir_name: "rustcodegraph",
        source_url: "bundled:rustcodegraph",
        markdown: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/skills/rustcodegraph/SKILL.md"
        )),
    },
];

/// 返回默认的下载策略版本号
///
/// 用于初始化技能下载策略配置时的版本字段。
///
/// # 返回值
///
/// 返回策略版本号 `1`。
pub(crate) fn default_policy_version() -> u32 {
    1
}

/// 返回默认预加载技能的别名映射表
///
/// 将 [`DEFAULT_PRELOADED_SKILL_SOURCES`] 转换为 `BTreeMap<String, String>` 格式，
/// 便于在运行时进行别名查找。
///
/// # 返回值
///
/// 返回一个 `BTreeMap`，键为技能别名，值为对应的源 URL。
///
/// # 示例
///
/// ```ignore
/// let aliases = default_preloaded_skill_aliases();
/// assert_eq!(aliases.get("create-task-plans"), Some(&"bundled:create-task-plans".to_string()));
/// ```
pub(crate) fn default_preloaded_skill_aliases() -> BTreeMap<String, String> {
    DEFAULT_PRELOADED_SKILL_SOURCES
        .iter()
        .map(|(alias, source)| ((*alias).to_string(), (*source).to_string()))
        .collect()
}

/// 返回默认版本号字符串
///
/// 用于初始化需要版本信息的配置项。
///
/// # 返回值
///
/// 返回默认版本号 `"0.1.0"`。
pub(crate) fn default_version() -> String {
    "0.1.0".to_string()
}
#[cfg(test)]
#[path = "constants_tests.rs"]
mod constants_tests;
