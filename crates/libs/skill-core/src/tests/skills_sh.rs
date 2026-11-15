//! skills.sh 技能源解析功能测试模块
//!
//! 本模块包含针对 skills.sh 平台技能源 URL 解析和目录名称规范化功能的单元测试。
//! 主要测试内容包括：
//! - skills.sh URL 格式的解析正确性
//! - 无效 URL 的拒绝处理
//! - 目录名称规范化逻辑
//!
//! # 测试覆盖
//!
//! - `parse_skills_sh_source` 函数：解析 skills.sh 格式的技能源 URL
//! - `normalize_skills_sh_dir_name` 函数：规范化技能目录名称

use super::*;

/// 测试 `parse_skills_sh_source` 函数对标准 skills.sh URL 格式的解析能力
///
/// # 测试场景
///
/// 1. 解析标准的 owner/repo/skill 格式 URL
/// 2. 解析带有尾部斜杠的 URL
///
/// # 验证点
///
/// - 正确提取 owner（所有者）字段
/// - 正确提取 repo（仓库名）字段
/// - 正确提取 skill（技能名）字段
/// - 支持带尾部斜杠的 URL 格式
#[test]
fn parse_skills_sh_source_accepts_owner_repo_skill_urls() {
    // 测试标准格式的 skills.sh URL
    let parsed = parse_skills_sh_source("https://skills.sh/vercel-labs/skills/find-skills")
        .expect("should parse skills.sh source");
    // 验证解析出的 owner 字段
    assert_eq!(parsed.owner, "vercel-labs");
    // 验证解析出的 repo 字段
    assert_eq!(parsed.repo, "skills");
    // 验证解析出的 skill 字段
    assert_eq!(parsed.skill, "find-skills");

    // 测试带尾部斜杠的 URL 格式
    let parsed_with_trailing =
        parse_skills_sh_source("https://skills.sh/anthropics/skills/skill-creator/")
            .expect("should parse trailing slash");
    // 验证带尾部斜杠的 URL 也能正确解析
    assert_eq!(parsed_with_trailing.owner, "anthropics");
    assert_eq!(parsed_with_trailing.repo, "skills");
    assert_eq!(parsed_with_trailing.skill, "skill-creator");
}

/// 测试 `parse_skills_sh_source` 函数对无效 URL 的拒绝处理
///
/// # 测试场景
///
/// 1. URL 路径段不足（缺少技能名）
/// 2. 非 skills.sh 域名的 URL
/// 3. 缺少协议前缀的 URL
///
/// # 验证点
///
/// - 所有无效应返回 `None` 而不是 panic 或返回错误数据
/// - 确保只接受完整的 skills.sh URL 格式
#[test]
fn parse_skills_sh_source_rejects_invalid_urls() {
    // 缺少技能名段（只有 owner/repo）
    assert!(parse_skills_sh_source("https://skills.sh/vercel-labs/skills").is_none());
    // 非 skills.sh 域名
    assert!(parse_skills_sh_source("https://example.com/vercel-labs/skills/find-skills").is_none());
    // 缺少协议前缀（https://）
    assert!(parse_skills_sh_source("skills.sh/vercel-labs/skills/find-skills").is_none());
}

/// 测试 `normalize_skills_sh_dir_name` 函数对目录名称的规范化处理
///
/// # 测试场景
///
/// 1. 已经是规范格式的名称（包含连字符）
/// 2. 包含大写字母和特殊字符的名称
///
/// # 验证点
///
/// - 保留现有的连字符（-）
/// - 将大写字母转换为小写
/// - 保留下划线等特殊字符
/// - 确保规范化后的名称符合目录命名规范
#[test]
fn normalize_skills_sh_dir_name_preserves_hyphens() {
    // 测试已经是规范格式的名称
    assert_eq!(normalize_skills_sh_dir_name("find-skills"), "find-skills");
    // 测试包含大写字母和特殊字符的名称
    assert_eq!(normalize_skills_sh_dir_name("Skill-Creator_2"), "skill-creator_2");
}
