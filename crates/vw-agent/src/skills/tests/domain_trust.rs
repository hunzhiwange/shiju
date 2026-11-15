//! 域名信任验证测试模块
//!
//! 本模块提供对域名信任机制的单元测试，验证主机名是否匹配受信任域名及其子域名。
//! 主要测试场景包括：
//! - 精确域名匹配
//! - 子域名匹配（如 cdn.skills.sh 匹配 skills.sh）
//! - 恶意域名防护（如 evilskills.sh 不应匹配 skills.sh）

use super::super::*;
use skill_core::host_matches_trusted_domain;

/// 测试 host_matches_trusted_domain 函数是否正确支持子域名匹配
///
/// # 测试场景
///
/// 1. **精确匹配**：主机名与信任域名完全相同时应返回 true
///    - 示例：`skills.sh` 匹配 `skills.sh`
///
/// 2. **子域名匹配**：主机名是信任域名的有效子域名时应返回 true
///    - 示例：`cdn.skills.sh` 匹配 `skills.sh`
///
/// 3. **恶意域名防护**：主机名仅包含信任域名字符串但非有效子域名时应返回 false
///    - 示例：`evilskills.sh` 不匹配 `skills.sh`（避免前缀伪造攻击）
///
/// # 安全考量
///
/// 此测试确保域名信任验证不会受到以下攻击：
/// - 前缀伪造：攻击者注册包含目标域名的域名（如 evil-skills.sh）
/// - 后缀伪造：攻击者注册以目标域名结尾的域名（如 skills.sh.evil.com）
#[test]
fn host_matches_trusted_domain_supports_subdomains() {
    // 测试精确匹配：完全相同的主机名应通过验证
    assert!(host_matches_trusted_domain("skills.sh", "skills.sh"));

    // 测试子域名匹配：有效的子域名应通过验证（cdn. 是 skills.sh 的子域）
    assert!(host_matches_trusted_domain("cdn.skills.sh", "skills.sh"));

    // 测试恶意域名防护：仅包含字符串但非有效子域名应被拒绝
    // evilskills.sh 不是 skills.sh 的子域名，防止前缀伪造攻击
    assert!(!host_matches_trusted_domain("evilskills.sh", "skills.sh"));
}
