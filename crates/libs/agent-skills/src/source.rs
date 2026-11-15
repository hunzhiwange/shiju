//! 技能源解析与处理模块
//!
//! 本模块提供对 skills.sh 平台技能源 URL 的解析、验证和规范化功能。
//! 支持将 skills.sh 格式的 URL 解析为结构化的源信息，便于后续的技能安装和管理。
//!
//! # 主要功能
//!
//! - 解析 skills.sh 平台的技能 URL
//! - 验证技能源的有效性和安全性
//! - 规范化技能目录名称
//! - 生成对应的 GitHub 仓库 URL

/// skills.sh 平台的主机地址常量
const SKILLS_SH_HOST: &str = "skills.sh";

/// 表示从 skills.sh 平台解析出的技能源信息
///
/// 该结构体封装了技能源的三部分关键信息：所有者、仓库名和技能名。
/// 这些信息用于定位和访问 GitHub 上的技能代码仓库。
///
/// # 字段
///
/// - `owner` - GitHub 仓库所有者（用户名或组织名）
/// - `repo` - GitHub 仓库名称
/// - `skill` - 技能在仓库中的名称
///
/// # 示例
///
/// ```ignore
/// use crate::source::SkillsShSource;
///
/// let source = SkillsShSource {
///     owner: "example-owner".to_string(),
///     repo: "skills-repo".to_string(),
///     skill: "my-skill".to_string(),
/// };
/// assert_eq!(
///     source.github_repo_url(),
///     "https://github.com/example-owner/skills-repo.git"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsShSource {
    /// GitHub 仓库所有者名称
    pub owner: String,
    /// GitHub 仓库名称
    pub repo: String,
    /// 技能名称
    pub skill: String,
}

impl SkillsShSource {
    /// 生成对应的 GitHub 仓库 Git 克隆 URL
    ///
    /// 根据技能源信息，构建标准的 GitHub HTTPS 克隆 URL。
    ///
    /// # 返回值
    ///
    /// 返回格式为 `https://github.com/{owner}/{repo}.git` 的字符串
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let source = SkillsShSource {
    ///     owner: "myorg".to_string(),
    ///     repo: "myskills".to_string(),
    ///     skill: "deploy".to_string(),
    /// };
    /// assert_eq!(
    ///     source.github_repo_url(),
    ///     "https://github.com/myorg/myskills.git"
    /// );
    /// ```
    pub fn github_repo_url(&self) -> String {
        format!("https://github.com/{}/{}.git", self.owner, self.repo)
    }
}

/// 规范化技能目录名称
///
/// 将输入字符串转换为适合作为目录名的安全格式。
/// 处理包括：转换为小写、过滤掉不安全的字符，只保留字母、数字、连字符和下划线。
///
/// # 参数
///
/// - `s` - 待规范化的原始字符串
///
/// # 返回值
///
/// 返回规范化后的安全目录名称字符串
///
/// # 示例
///
/// ```ignore
/// use crate::source::normalize_skills_sh_dir_name;
///
/// assert_eq!(normalize_skills_sh_dir_name("My-Skill_123"), "my-skill_123");
/// assert_eq!(normalize_skills_sh_dir_name("Test@Skill#Name"), "testskillname");
/// ```
pub fn normalize_skills_sh_dir_name(s: &str) -> String {
    s.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

/// 解析 skills.sh 平台的技能源 URL
///
/// 从标准的 skills.sh URL 中提取所有者、仓库和技能信息。
/// 同时进行安全性验证，防止路径遍历攻击。
///
/// # URL 格式
///
/// 支持的 URL 格式：`https://skills.sh/{owner}/{repo}/{skill}[?query][#fragment]`
///
/// # 参数
///
/// - `source` - skills.sh 平台的技能源 URL 字符串
///
/// # 返回值
///
/// - `Some(SkillsShSource)` - 解析成功，返回结构化的源信息
/// - `None` - URL 格式不正确或包含不安全字符
///
/// # 安全性
///
/// 该函数会拒绝包含以下危险模式的 URL：
/// - 路径遍历字符 `..`
/// - 反斜杠 `\`
///
/// # 示例
///
/// ```ignore
/// use crate::source::parse_skills_sh_source;
///
/// let source = parse_skills_sh_source("https://skills.sh/myorg/myrepo/myskill");
/// assert!(source.is_some());
/// let info = source.unwrap();
/// assert_eq!(info.owner, "myorg");
/// assert_eq!(info.repo, "myrepo");
/// assert_eq!(info.skill, "myskill");
///
/// // 包含查询参数的 URL
/// let source = parse_skills_sh_source("https://skills.sh/org/repo/skill?version=1.0");
/// assert!(source.is_some());
///
/// // 无效的 URL
/// let invalid = parse_skills_sh_source("https://github.com/org/repo");
/// assert!(invalid.is_none());
/// ```
pub fn parse_skills_sh_source(source: &str) -> Option<SkillsShSource> {
    // 移除 "https://" 前缀，失败则返回 None
    let rest = source.strip_prefix("https://")?;
    // 移除 "skills.sh" 主机名，失败则返回 None
    let rest = rest.strip_prefix(SKILLS_SH_HOST)?;

    // 提取路径部分：移除开头的 '/'，然后按 '?' 或 '#' 分割取第一部分
    let path = rest.trim_start_matches('/').split(&['?', '#'][..]).next().unwrap_or("");

    // 将路径按 '/' 分割为多个段，并过滤掉空段
    let mut segments = path.split('/').filter(|part| !part.trim().is_empty());

    // 依次提取三个必需的路径段：owner、repo、skill
    let owner = segments.next()?;
    let repo = segments.next()?;
    let skill = segments.next()?;

    // 安全性检查：拒绝包含路径遍历字符或反斜杠的输入
    // 防止目录遍历攻击和不规范的路径格式
    if owner.contains("..")
        || repo.contains("..")
        || skill.contains("..")
        || owner.contains('\\')
        || repo.contains('\\')
        || skill.contains('\\')
    {
        return None;
    }

    // 构建并返回解析后的技能源信息
    Some(SkillsShSource {
        owner: owner.to_string(),
        repo: repo.to_string(),
        skill: skill.to_string(),
    })
}

/// 判断给定的字符串是否为有效的 skills.sh 源 URL
///
/// 这是一个便捷函数，通过尝试解析来验证 URL 的有效性。
///
/// # 参数
///
/// - `source` - 待检查的源 URL 字符串
///
/// # 返回值
///
/// - `true` - 是有效的 skills.sh 源 URL
/// - `false` - 不是有效的 skills.sh 源 URL
///
/// # 示例
///
/// ```ignore
/// use crate::source::is_skills_sh_source;
///
/// assert!(is_skills_sh_source("https://skills.sh/org/repo/skill"));
/// assert!(!is_skills_sh_source("https://github.com/org/repo"));
/// assert!(!is_skills_sh_source("invalid-url"));
/// ```
pub fn is_skills_sh_source(source: &str) -> bool {
    parse_skills_sh_source(source).is_some()
}
#[cfg(test)]
#[path = "source_tests.rs"]
mod source_tests;
