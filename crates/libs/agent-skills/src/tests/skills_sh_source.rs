//! skills.sh 源解析功能测试模块
//!
//! 本模块包含对 skills.sh URL 解析和目录名规范化功能的单元测试。
//! 主要测试内容包括：
//! - 解析符合格式的 skills.sh URL（包含 owner/repo/skill 三部分）
//! - 拒绝无效的 URL 格式
//! - 规范化技能目录名称（保留连字符，转换为小写）

use crate::source::{normalize_skills_sh_dir_name, parse_skills_sh_source};

/// 测试解析标准格式的 skills.sh URL
///
/// 验证解析器能够正确提取 URL 中的三个组成部分：
/// - owner（所有者）
/// - repo（仓库名，通常为 "skills"）
/// - skill（技能名称）
///
/// 同时验证带有尾部斜杠的 URL 也能正确解析。
#[test]
fn parse_skills_sh_source_accepts_owner_repo_skill_urls() {
    // 测试标准格式的 skills.sh URL 解析
    let parsed = parse_skills_sh_source("https://skills.sh/vercel-labs/skills/find-skills")
        .expect("should parse skills.sh source");
    // 验证解析出的各部分是否正确
    assert_eq!(parsed.owner, "vercel-labs");
    assert_eq!(parsed.repo, "skills");
    assert_eq!(parsed.skill, "find-skills");

    // 测试带有尾部斜杠的 URL，确保解析器能正确处理
    let parsed_with_trailing =
        parse_skills_sh_source("https://skills.sh/anthropics/skills/skill-creator/")
            .expect("should parse trailing slash");
    // 验证带斜杠的 URL 也能正确解析各部分
    assert_eq!(parsed_with_trailing.owner, "anthropics");
    assert_eq!(parsed_with_trailing.repo, "skills");
    assert_eq!(parsed_with_trailing.skill, "skill-creator");
}

/// 测试解析器拒绝无效的 URL 格式
///
/// 验证以下无效 URL 会被拒绝：
/// - 缺少 skill 部分的 URL（只有 owner/repo）
/// - 非 skills.sh 域名的 URL
/// - 缺少协议头的 URL
#[test]
fn parse_skills_sh_source_rejects_invalid_urls() {
    // 缺少 skill 名称部分，应返回 None
    assert!(parse_skills_sh_source("https://skills.sh/vercel-labs/skills").is_none());
    // 域名不是 skills.sh，应返回 None
    assert!(parse_skills_sh_source("https://example.com/vercel-labs/skills/find-skills").is_none());
    // 缺少 https:// 协议头，应返回 None
    assert!(parse_skills_sh_source("skills.sh/vercel-labs/skills/find-skills").is_none());
}

/// 测试目录名规范化函数保留连字符
///
/// 验证规范化函数的行为：
/// - 保留名称中的连字符（-）
/// - 将大写字母转换为小写
/// - 保留下划线（_）和数字
#[test]
fn normalize_skills_sh_dir_name_preserves_hyphens() {
    // 测试纯连字符名称，应保持不变
    assert_eq!(normalize_skills_sh_dir_name("find-skills"), "find-skills");
    // 测试包含大写字母、连字符、下划线和数字的名称，应转换为小写但保留特殊字符
    assert_eq!(normalize_skills_sh_dir_name("Skill-Creator_2"), "skill-creator_2");
}
