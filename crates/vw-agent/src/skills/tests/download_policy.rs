//! # 技能下载策略测试模块
//!
//! 本模块包含针对技能下载策略（`SkillDownloadPolicy`）及其相关函数的单元测试。
//!
//! ## 测试范围
//!
//! - 验证默认下载策略是否包含所需的预加载技能源别名
//! - 验证技能源别名解析函数的行为是否符合预期优先级
//!
//! ## 核心概念
//!
//! - **下载策略**：定义技能从何处下载的配置规则，包含源别名映射
//! - **源别名**：将简短的技能名称映射到完整的下载 URL 或本地源标识
//! - **预加载源**：系统默认提供的一组常用技能源，开箱即用

use super::super::*;
use crate::app::agent::skill::{SkillDownloadPolicy, resolve_skill_source_alias};

/// 测试默认下载策略是否包含必需的预加载技能源
///
/// # 验证点
///
/// 1. `create-task-plans` 别名应映射到本地打包源标识
///
/// # 设计意图
///
/// 这些预加载的别名确保用户无需手动配置即可使用核心技能。
/// 如果这些默认值缺失，将导致基础功能不可用。
///
/// # 示例场景
///
/// 当用户在配置中引用 `create-task-plans` 时，系统应自动解析为：
/// `bundled:create-task-plans`
#[test]
fn default_download_policy_contains_required_preloaded_sources() {
    // 获取默认下载策略实例
    let policy = SkillDownloadPolicy::default();

    // 验证 create-task-plans 别名指向正确的本地打包源
    assert_eq!(
        policy.aliases.get("create-task-plans"),
        Some(&"bundled:create-task-plans".to_string())
    );
    assert_eq!(
        policy.aliases.get("do-task-plans"),
        Some(&"bundled:do-task-plans".to_string())
    );
    assert_eq!(
        policy.aliases.get("rustcodegraph"),
        Some(&"bundled:rustcodegraph".to_string())
    );
}

/// 测试技能源别名解析函数的优先级行为
///
/// # 验证点
///
/// 1. **用户自定义别名优先**：用户配置的别名应覆盖同名默认别名
/// 2. **默认别名次之**：未覆盖的默认别名仍可正常解析
/// 3. **完整 URL 直接返回**：如果输入已是完整 URL，则无需解析直接返回
///
/// # 设计意图
///
/// 此优先级设计允许用户：
/// - 保留默认别名以获得开箱即用的体验
/// - 通过自定义同名别名覆盖默认配置
/// - 直接使用完整 URL 而不受别名系统影响
///
/// # 示例场景
///
/// ```ignore
/// // 场景 1：用户自定义别名优先
/// resolve_skill_source_alias("custom", &policy)
/// // 返回用户配置的 URL
///
/// // 场景 2：默认别名正常工作
/// resolve_skill_source_alias("create-task-plans", &policy)
/// // 返回默认的本地打包源标识
///
/// // 场景 3：完整 URL 直接透传
/// resolve_skill_source_alias("https://example.com/skill.zip", &policy)
/// // 返回原 URL 不变
/// ```
#[test]
fn resolve_skill_source_alias_prefers_user_and_default_aliases() {
    // 创建包含自定义别名覆盖的下载策略
    let mut policy = SkillDownloadPolicy::default();

    // 添加用户自定义别名，用于测试优先级覆盖
    policy.aliases.insert("custom".to_string(), "https://skills.sh/acme/skills/custom".to_string());

    // 验证点 1：用户自定义别名应被正确解析
    assert_eq!(
        resolve_skill_source_alias("custom", &policy),
        "https://skills.sh/acme/skills/custom".to_string()
    );

    // 验证点 2：未被覆盖的默认别名仍可正常工作
    assert_eq!(
        resolve_skill_source_alias("create-task-plans", &policy),
        "bundled:create-task-plans".to_string()
    );

    // 验证点 3：完整 URL 应直接返回，不进行别名解析
    assert_eq!(
        resolve_skill_source_alias("https://example.com/skill.zip", &policy),
        "https://example.com/skill.zip".to_string()
    );
}
