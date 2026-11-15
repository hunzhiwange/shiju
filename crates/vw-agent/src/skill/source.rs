//! 技能源（Source）解析与规范化模块
//!
//! 本模块提供技能来源 URL 的解析、验证和规范化功能，支持多种源类型：
//! - 直接 Git 仓库 URL（https/http/ssh/git 协议）
//! - skills.sh 平台源（格式：`https://skills.sh/{owner}/{repo}/{skill}`）
//!
//! 主要职责：
//! - 从 URL 中提取主机名用于信任检查
//! - 解析并验证 skills.sh 源格式
//! - 规范化域名和目录名称

use skill_core::constants::SKILLS_SH_HOST;
use crate::app::agent::skill::types::SkillsShSource;
use std::collections::HashSet;

/// 从 URL 中提取并规范化主机名
///
/// 支持多种 URL 格式，包括：
/// - `https://` / `http://` 协议
/// - `ssh://` / `git://` 协议
/// - `zip:` 前缀（会被自动移除）
///
/// # 参数
///
/// * `url` - 待解析的 URL 字符串
///
/// # 返回值
///
/// - `Some(String)` - 成功提取的规范化主机名
/// - `None` - URL 格式无效或无法提取主机名
///
/// # 示例
///
/// ```ignore
/// let host = extract_link_host("https://github.com/user/repo");
/// assert_eq!(host, Some("github.com".to_string()));
///
/// let host = extract_link_host("ssh://git@github.com:22/user/repo");
/// assert_eq!(host, Some("github.com".to_string()));
/// ```
pub(crate) fn extract_link_host(url: &str) -> Option<String> {
    // 移除 "zip:" 前缀（如果存在），用于处理压缩包 URL
    let trimmed = url.strip_prefix("zip:").unwrap_or(url);

    // 尝试剥离各种协议前缀，获取路径部分
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .or_else(|| trimmed.strip_prefix("ssh://"))
        .or_else(|| trimmed.strip_prefix("git://"))?;

    // 按 '/', '?', '#' 分割，取第一部分作为主机（去除路径、查询参数和锚点）
    let host_part = rest.split(&['/', '?', '#'][..]).next().unwrap_or("");

    // 移除 SSH 用户信息（如 git@github.com -> github.com）
    let host_part = host_part.rsplit('@').next().unwrap_or(host_part);

    // 移除端口号（如 github.com:22 -> github.com）
    let host = host_part.split(':').next().unwrap_or("");

    // 规范化域名并返回
    let normalized = normalize_domain_entry(host);
    if normalized.is_empty() { None } else { Some(normalized) }
}

/// 从源字符串中提取所有需要信任检查的 URL
///
/// 此函数会解析源字符串，返回一个去重的 URL 列表，用于后续的信任验证。
/// 支持直接 URL 和 skills.sh 源格式。
///
/// # 参数
///
/// * `source` - 技能源字符串，可以是直接的 Git URL 或 skills.sh 格式
///
/// # 返回值
///
/// 返回去重后的 URL 向量，每个 URL 都是独立的信任检查目标
///
/// # 示例
///
/// ```ignore
/// // 直接 URL
/// let urls = source_urls_for_trust_check("https://github.com/user/repo");
/// assert_eq!(urls, vec!["https://github.com/user/repo".to_string()]);
///
/// // skills.sh 源会被转换为对应的 GitHub 仓库 URL
/// let urls = source_urls_for_trust_check("https://skills.sh/owner/repo/skill");
/// // 返回对应的 GitHub 仓库 URL
/// ```
pub(crate) fn source_urls_for_trust_check(source: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut seen = HashSet::new();

    // 内部闭包：确保只添加唯一的 URL
    let mut push_unique = |url: String| {
        if seen.insert(url.clone()) {
            urls.push(url);
        }
    };

    // 如果是直接的 Git 协议 URL，直接添加到列表
    if source.starts_with("https://")
        || source.starts_with("http://")
        || source.starts_with("ssh://")
        || source.starts_with("git://")
    {
        push_unique(source.to_string());
    }

    // 如果是 skills.sh 源，解析并添加其对应的 GitHub 仓库 URL
    if let Some(skills_source) = parse_skills_sh_source(source) {
        push_unique(skills_source.github_repo_url());
    }

    urls
}

/// 检查源字符串是否为 skills.sh 格式
///
/// # 参数
///
/// * `source` - 待检查的源字符串
///
/// # 返回值
///
/// - `true` - 是有效的 skills.sh 源格式
/// - `false` - 不是 skills.sh 源格式
///
/// # 示例
///
/// ```ignore
/// assert!(is_skills_sh_source("https://skills.sh/owner/repo/skill"));
/// assert!(!is_skills_sh_source("https://github.com/user/repo"));
/// ```
pub(crate) fn is_skills_sh_source(source: &str) -> bool {
    parse_skills_sh_source(source).is_some()
}

/// 解析 skills.sh 源字符串
///
/// 将 skills.sh 格式的 URL 解析为结构化的 `SkillsShSource` 对象。
/// 格式：`https://skills.sh/{owner}/{repo}/{skill}`
///
/// # 参数
///
/// * `source` - skills.sh 格式的源字符串
///
/// # 返回值
///
/// - `Some(SkillsShSource)` - 成功解析后的结构化数据
/// - `None` - 格式无效或包含不安全字符
///
/// # 安全性
///
/// 此函数会拒绝包含以下字符的输入，以防止路径遍历攻击：
/// - `..`（双点）
/// - `\`（反斜杠）
///
/// # 示例
///
/// ```ignore
/// let source = parse_skills_sh_source("https://skills.sh/openai/coding-agent/python");
/// assert!(source.is_some());
/// let s = source.unwrap();
/// assert_eq!(s.owner, "openai");
/// assert_eq!(s.repo, "coding-agent");
/// assert_eq!(s.skill, "python");
/// ```
pub(crate) fn parse_skills_sh_source(source: &str) -> Option<SkillsShSource> {
    // 验证并剥离 "https://" 前缀
    let rest = source.strip_prefix("https://")?;

    // 验证并剥离 skills.sh 主机名
    let rest = rest.strip_prefix(SKILLS_SH_HOST)?;

    // 提取路径部分，去除查询参数和锚点
    let path = rest.trim_start_matches('/').split(&['?', '#'][..]).next().unwrap_or("");

    // 按路径分割并过滤空段
    let mut segments = path.split('/').filter(|part| !part.trim().is_empty());

    // 解析三个必需的路径段：owner/repo/skill
    let owner = segments.next()?;
    let repo = segments.next()?;
    let skill = segments.next()?;

    // 安全检查：拒绝路径遍历字符
    if owner.contains("..")
        || repo.contains("..")
        || skill.contains("..")
        || owner.contains('\\')
        || repo.contains('\\')
        || skill.contains('\\')
    {
        return None;
    }

    Some(SkillsShSource {
        owner: owner.to_string(),
        repo: repo.to_string(),
        skill: skill.to_string(),
    })
}

/// 规范化 skills.sh 目录名称
///
/// 将字符串转换为适合作为目录名的安全格式：
/// - 转换为小写
/// - 仅保留字母、数字、连字符和下划线
///
/// # 参数
///
/// * `s` - 原始字符串
///
/// # 返回值
///
/// 规范化后的安全目录名字符串
///
/// # 示例
///
/// ```ignore
/// assert_eq!(normalize_skills_sh_dir_name("MySkill-123"), "myskill-123");
/// assert_eq!(normalize_skills_sh_dir_name("Test@Skill!"), "testskill");
/// ```
pub(crate) fn normalize_skills_sh_dir_name(s: &str) -> String {
    s.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// 规范化域名条目
///
/// 将原始域名字符串规范化为统一格式：
/// - 去除空白并转换为小写
/// - 移除协议前缀
/// - 移除路径、查询参数和锚点
/// - 移除通配符前缀
/// - 移除端口号
///
/// # 参数
///
/// * `raw` - 原始域名或 URL 片段
///
/// # 返回值
///
/// 规范化后的纯域名
fn normalize_domain_entry(raw: &str) -> String {
    // 去除首尾空白并转换为小写
    let mut s = raw.trim().to_ascii_lowercase();

    if s.is_empty() {
        return s;
    }

    // 移除协议前缀（防御性处理，通常外层已剥离）
    if let Some(rest) = s.strip_prefix("https://") {
        s = rest.to_string();
    } else if let Some(rest) = s.strip_prefix("http://") {
        s = rest.to_string();
    }

    // 移除路径、查询参数和锚点部分
    s = s.split(&['/', '?', '#'][..]).next().unwrap_or("").trim().to_string();

    // 移除通配符前缀（如 *.example.com -> example.com）
    s = s.trim_start_matches("*.").trim_start_matches('.').to_string();

    // 移除端口号
    if let Some((host, _port)) = s.split_once(':') {
        return host.to_string();
    }

    s
}
#[cfg(test)]
#[path = "source_tests.rs"]
mod source_tests;
