//! 技能下载策略测试模块
//!
//! 本模块包含针对技能下载策略 (`SkillDownloadPolicy`) 及相关功能的单元测试，
//! 主要验证以下方面：
//!
//! - 默认下载策略是否包含必需的预加载源
//! - 技能源别名解析是否正确优先使用用户自定义和默认别名
//! - 受信任域名匹配是否正确支持子域名
//!
//! 这些测试确保技能下载机制的安全性和可用性。

use super::super::*;

/// 测试默认下载策略包含必需的预加载源
///
/// 验证 `SkillDownloadPolicy::default()` 返回的策略中包含以下必需的别名映射：
/// - `create-task-plans` -> `bundled:create-task-plans`
///
/// 这些预加载源是系统核心功能的依赖项，必须始终可用。
#[test]
fn default_download_policy_contains_required_preloaded_sources() {
    let policy = SkillDownloadPolicy::default();

    // 验证 create-task-plans 别名映射正确
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

/// 测试技能源别名解析优先级
///
/// 验证 `resolve_skill_source_alias` 函数的解析行为：
///
/// # 解析优先级（从高到低）
///
/// 1. **用户自定义别名** - 用户在策略中显式配置的别名
/// 2. **默认别名** - 系统预置的标准别名（如 `create-task-plans`）
/// 3. **原始 URL** - 如果输入不是别名，直接返回原始 URL
///
/// # 测试场景
///
/// - 自定义别名 `custom` 应解析为用户配置的 URL
/// - 默认别名 `create-task-plans` 应解析为预置的源标识
/// - 直接的 URL 应原样返回，不做任何转换
#[test]
fn resolve_skill_source_alias_prefers_user_and_default_aliases() {
    // 创建一个包含用户自定义别名的策略
    let mut policy = SkillDownloadPolicy::default();
    policy.aliases.insert("custom".to_string(), "https://skills.sh/acme/skills/custom".to_string());

    // 验证用户自定义别名优先解析
    assert_eq!(
        resolve_skill_source_alias("custom", &policy),
        "https://skills.sh/acme/skills/custom".to_string()
    );

    // 验证默认别名仍然可以正常解析
    assert_eq!(
        resolve_skill_source_alias("create-task-plans", &policy),
        "bundled:create-task-plans".to_string()
    );

    // 验证直接的 URL 原样返回
    assert_eq!(
        resolve_skill_source_alias("https://example.com/skill.zip", &policy),
        "https://example.com/skill.zip".to_string()
    );
}

/// 测试受信任域名匹配支持子域名
///
/// 验证 `host_matches_trusted_domain` 函数的域名匹配逻辑：
///
/// # 匹配规则
///
/// - **精确匹配**: 主机名完全等于受信任域名时匹配成功
/// - **子域名匹配**: 主机名为受信任域名的子域名时匹配成功
/// - **非子域名排除**: 主机名仅包含受信任域名字符串但不是子域名时匹配失败
///
/// # 测试场景
///
/// - `skills.sh` 精确匹配 `skills.sh` ✓
/// - `cdn.skills.sh` 作为子域名匹配 `skills.sh` ✓
/// - `evilskills.sh` 不匹配 `skills.sh`（非子域名关系）✗
#[test]
fn host_matches_trusted_domain_supports_subdomains() {
    // 精确匹配：主机名完全等于受信任域名
    assert!(host_matches_trusted_domain("skills.sh", "skills.sh"));

    // 子域名匹配：主机名是受信任域名的子域名
    assert!(host_matches_trusted_domain("cdn.skills.sh", "skills.sh"));

    // 非子域名：主机名包含域名字符串但不是子域名关系，应拒绝
    assert!(!host_matches_trusted_domain("evilskills.sh", "skills.sh"));
}
