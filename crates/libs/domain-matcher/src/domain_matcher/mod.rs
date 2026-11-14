//! 域名匹配器模块
//!
//! 本模块提供了域名模式匹配功能，用于检查目标域名是否属于需要 OTP 验证的保护域。
//! 支持通配符模式匹配，并提供预定义的敏感域名分类（银行、医疗、政府、身份提供商）。
//!
//! # 主要功能
//!
//! - 域名规范化处理（去除协议、端口、路径等）
//! - 通配符模式匹配（支持 `*` 通配符）
//! - 预定义的敏感域名分类（banking、medical、government、identity_providers）
//! - 自定义域名模式验证与匹配
//!
//! # 使用示例
//!
//! ```rust
//! use vibe_agent::security::DomainMatcher;
//!
//! // 创建匹配器，包含自定义域名和分类
//! let matcher = DomainMatcher::new(
//!     &["*.example.com".to_string()],
//!     &["banking".to_string()]
//! )?;
//!
//! // 检查域名是否在保护列表中
//! assert!(matcher.is_gated("www.chase.com"));
//! assert!(matcher.is_gated("api.example.com"));
//! assert!(!matcher.is_gated("google.com"));
//! ```

use anyhow::{Result, bail};
use std::collections::BTreeSet;

/// 银行与金融服务域名列表
///
/// 包含主流银行、支付平台和投资服务的域名模式。
/// 所有模式使用通配符前缀 `*.` 以匹配所有子域名。
const BANKING_DOMAINS: &[&str] = &[
    "*.chase.com",         // 摩根大通银行
    "*.bankofamerica.com", // 美国银行
    "*.wellsfargo.com",    // 富国银行
    "*.fidelity.com",      // 富达投资
    "*.schwab.com",        // 嘉信理财
    "*.venmo.com",         // Venmo 支付
    "*.paypal.com",        // PayPal 支付
    "*.robinhood.com",     // Robinhood 投资平台
    "*.coinbase.com",      // Coinbase 加密货币交易所
];

/// 医疗健康域名列表
///
/// 包含电子病历系统和患者门户的域名模式。
/// 这些域名通常涉及敏感的健康信息（PHI）。
const MEDICAL_DOMAINS: &[&str] = &[
    "*.mychart.com",      // Epic MyChart 患者门户
    "*.epic.com",         // Epic 医疗系统
    "*.patient.portal.*", // 通用患者门户模式
    "*.healthrecords.*",  // 通用健康记录模式
];

/// 政府机构域名列表
///
/// 包含美国政府机构的域名模式。
/// 这些域名通常涉及税务、社会保障、身份验证等敏感服务。
const GOVERNMENT_DOMAINS: &[&str] = &[
    "*.ssa.gov",   // 社会保障局
    "*.irs.gov",   // 美国国税局
    "*.login.gov", // 联邦登录服务
    "*.id.me",     // ID.me 身份验证服务
];

/// 身份提供商域名列表
///
/// 包含主要身份提供商的精确域名（不含通配符）。
/// 这些域名用于 OAuth/OIDC 身份验证流程。
const IDENTITY_PROVIDER_DOMAINS: &[&str] = &[
    "accounts.google.com",       // Google 账户服务
    "login.microsoftonline.com", // Microsoft 在线登录
    "appleid.apple.com",         // Apple ID
];

/// 域名分类映射表
///
/// 将分类名称映射到对应的域名模式列表。
/// 用于支持通过分类名称批量添加保护域名。
///
/// # 支持的分类
///
/// - `"banking"`: 银行与金融服务
/// - `"medical"`: 医疗健康服务
/// - `"government"`: 政府机构服务
/// - `"identity_providers"`: 身份提供商服务
const DOMAIN_CATEGORIES: &[(&str, &[&str])] = &[
    ("banking", BANKING_DOMAINS),
    ("medical", MEDICAL_DOMAINS),
    ("government", GOVERNMENT_DOMAINS),
    ("identity_providers", IDENTITY_PROVIDER_DOMAINS),
];

/// 域名匹配器
///
/// 用于检查给定域名是否属于需要 OTP 验证的保护域名列表。
/// 支持通配符模式匹配和预定义分类。
///
/// # 示例
///
/// ```rust
/// let matcher = DomainMatcher::new(
///     &["*.bank.com".to_string()],
///     &["banking".to_string()]
/// )?;
///
/// // 检查域名是否被保护
/// if matcher.is_gated("www.mybank.com") {
///     println!("此域名需要 OTP 验证");
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct DomainMatcher {
    /// 已规范化的域名模式列表
    ///
    /// 所有模式在初始化时经过规范化处理，确保格式一致。
    /// 使用 `BTreeSet` 去重并排序，最终转换为 `Vec` 存储。
    patterns: Vec<String>,
}

impl DomainMatcher {
    /// 创建新的域名匹配器
    ///
    /// 根据提供的自定义域名列表和分类名称构建匹配器。
    /// 所有域名在添加前都会进行规范化处理。
    ///
    /// # 参数
    ///
    /// - `gated_domains`: 自定义保护域名模式列表
    ///   - 支持通配符 `*`（如 `*.example.com`）
    ///   - 会被自动转换为小写并验证格式
    ///
    /// - `categories`: 预定义分类名称列表
    ///   - 可选值: `"banking"`, `"medical"`, `"government"`, `"identity_providers"`
    ///   - 不区分大小写
    ///   - 分类中的域名会自动展开添加
    ///
    /// # 返回值
    ///
    /// 返回初始化成功的 `DomainMatcher` 实例，或包含错误信息的 `Result`。
    ///
    /// # 错误
    ///
    /// - 域名模式格式无效（如空字符串、连续点号、非法字符等）
    /// - 分类名称未知
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 仅使用自定义域名
    /// let matcher = DomainMatcher::new(
    ///     &["*.internal.com".to_string()],
    ///     &[]
    /// )?;
    ///
    /// // 使用分类
    /// let matcher = DomainMatcher::new(
    ///     &[],
    ///     &["banking".to_string(), "government".to_string()]
    /// )?;
    ///
    /// // 混合使用
    /// let matcher = DomainMatcher::new(
    ///     &["secure.myservice.com".to_string()],
    ///     &["banking".to_string()]
    /// )?;
    /// ```
    pub fn new(gated_domains: &[String], categories: &[String]) -> Result<Self> {
        let mut set = BTreeSet::new();

        // 添加自定义域名，并进行规范化处理
        for domain in gated_domains {
            set.insert(normalize_pattern(domain)?);
        }

        // 展开并添加分类中的域名
        for domain in Self::expand_categories(categories)? {
            set.insert(domain);
        }

        // 将 Set 转换为 Vec 存储，利用 BTreeSet 的排序特性
        Ok(Self { patterns: set.into_iter().collect() })
    }

    /// 获取所有已注册的域名模式
    ///
    /// 返回匹配器中所有规范化后的域名模式列表。
    /// 列表按字母顺序排序且无重复项。
    ///
    /// # 返回值
    ///
    /// 域名模式字符串切片引用
    ///
    /// # 示例
    ///
    /// ```rust
    /// let matcher = DomainMatcher::new(
    ///     &["a.example.com".to_string(), "b.example.com".to_string()],
    ///     &[]
    /// )?;
    ///
    /// let patterns = matcher.patterns();
    /// assert_eq!(patterns.len(), 2);
    /// ```
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }

    /// 检查域名是否在保护列表中
    ///
    /// 判断给定的域名是否匹配任意一个已注册的域名模式。
    /// 输入域名会自动进行规范化处理（去除协议、端口、路径等）。
    ///
    /// # 参数
    ///
    /// - `domain`: 待检查的域名
    ///   - 可以包含协议前缀（如 `https://example.com`）
    ///   - 可以包含端口号（如 `example.com:8080`）
    ///   - 可以包含路径（如 `example.com/path`）
    ///   - 会自动转换为小写进行比较
    ///
    /// # 返回值
    ///
    /// - `true`: 域名匹配至少一个模式，需要 OTP 验证
    /// - `false`: 域名不匹配任何模式，或输入无效
    ///
    /// # 示例
    ///
    /// ```rust
    /// let matcher = DomainMatcher::new(
    ///     &["*.example.com".to_string()],
    ///     &["banking".to_string()]
    /// )?;
    ///
    /// // 通配符匹配
    /// assert!(matcher.is_gated("www.example.com"));
    /// assert!(matcher.is_gated("api.example.com"));
    ///
    /// // 带协议和端口
    /// assert!(matcher.is_gated("https://www.example.com:443"));
    ///
    /// // 预定义分类中的域名
    /// assert!(matcher.is_gated("www.chase.com"));
    ///
    /// // 不匹配的域名
    /// assert!(!matcher.is_gated("other.com"));
    /// ```
    pub fn is_gated(&self, domain: &str) -> bool {
        // 先对输入域名进行规范化处理
        let Some(normalized_domain) = normalize_domain(domain) else {
            // 规范化失败（如空字符串），认为不在保护列表中
            return false;
        };

        // 检查是否匹配任意模式
        self.patterns.iter().any(|pattern| domain_matches_pattern(pattern, &normalized_domain))
    }

    /// 展开域名分类为具体的域名列表
    ///
    /// 将分类名称转换为对应的域名模式列表。
    /// 可用于查看某个分类包含哪些域名。
    ///
    /// # 参数
    ///
    /// - `categories`: 分类名称列表
    ///   - 支持的分类: `"banking"`, `"medical"`, `"government"`, `"identity_providers"`
    ///   - 名称不区分大小写
    ///   - 前后空格会被自动去除
    ///
    /// # 返回值
    ///
    /// 成功时返回展开后的域名模式列表。
    ///
    /// # 错误
    ///
    /// 如果包含未知的分类名称，返回错误信息，其中包含所有已知分类名称。
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 展开单个分类
    /// let domains = DomainMatcher::expand_categories(&["banking".to_string()])?;
    /// assert!(domains.contains(&"*.chase.com".to_string()));
    ///
    /// // 展开多个分类
    /// let domains = DomainMatcher::expand_categories(
    ///     &["banking".to_string(), "medical".to_string()]
    /// )?;
    /// assert!(domains.len() > 10);
    ///
    /// // 未知分类会报错
    /// let result = DomainMatcher::expand_categories(&["unknown".to_string()]);
    /// assert!(result.is_err());
    /// ```
    pub fn expand_categories(categories: &[String]) -> Result<Vec<String>> {
        let mut expanded = Vec::new();

        // 遍历每个分类名称
        for category in categories {
            // 规范化分类名称：去除空格并转为小写
            let normalized = category.trim().to_ascii_lowercase();

            // 在预定义分类表中查找
            let Some((_, domains)) =
                DOMAIN_CATEGORIES.iter().find(|(name, _)| *name == normalized.as_str())
            else {
                // 未找到分类，构建错误信息
                let known =
                    DOMAIN_CATEGORIES.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(", ");
                bail!("Unknown OTP domain category '{category}'. Known categories: {known}");
            };

            // 将分类中的所有域名添加到结果列表
            expanded.extend(domains.iter().map(|domain| (*domain).to_string()));
        }
        Ok(expanded)
    }

    /// 验证域名模式格式是否有效
    ///
    /// 检查给定的域名模式是否符合格式要求。
    /// 用于在添加自定义域名前进行预验证。
    ///
    /// # 参数
    ///
    /// - `pattern`: 待验证的域名模式
    ///
    /// # 返回值
    ///
    /// 成功时返回 `Ok(())`，失败时返回包含错误详情的 `Err`。
    ///
    /// # 验证规则
    ///
    /// - 不能为空
    /// - 特殊值 `"*"` 表示匹配所有域名（危险，慎用）
    /// - 不能以 `.` 开头或结尾
    /// - 不能包含连续的 `..`
    /// - 不能包含连续的 `**`
    /// - 只能包含小写字母、数字、`.`、`-`、`*`
    /// - 不能有空标签（如 `example..com`）
    /// - `*.` 开头的模式后必须有内容
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 有效的模式
    /// DomainMatcher::validate_pattern("*.example.com")?;
    /// DomainMatcher::validate_pattern("www.example.com")?;
    /// DomainMatcher::validate_pattern("example.com")?;
    /// DomainMatcher::validate_pattern("*")?;  // 匹配所有
    ///
    /// // 无效的模式
    /// assert!(DomainMatcher::validate_pattern("").is_err());
    /// assert!(DomainMatcher::validate_pattern(".example.com").is_err());
    /// assert!(DomainMatcher::validate_pattern("example..com").is_err());
    /// assert!(DomainMatcher::validate_pattern("Example.COM").is_err()); // 大写
    /// ```
    pub fn validate_pattern(pattern: &str) -> Result<()> {
        // 通过尝试规范化来验证，如果规范化成功则模式有效
        let _ = normalize_pattern(pattern)?;
        Ok(())
    }
}

/// 规范化原始域名输入
///
/// 将用户输入的域名转换为标准格式，去除协议、端口、路径等额外信息。
/// 主要用于处理来自浏览器或应用的实际 URL 输入。
///
/// # 处理步骤
///
/// 1. 去除首尾空白字符
/// 2. 转换为小写
/// 3. 去除协议前缀（如 `https://`）
/// 4. 去除路径、查询参数、片段标识符
/// 5. 去除用户信息部分（如 `user@`）
/// 6. 去除端口号
/// 7. 去除末尾的点号
///
/// # 参数
///
/// - `raw`: 原始域名或 URL 字符串
///
/// # 返回值
///
/// - `Some(String)`: 规范化后的纯域名
/// - `None`: 输入为空或规范化后为空
///
/// # 示例
///
/// ```rust
/// assert_eq!(normalize_domain("https://www.example.com/path"), Some("www.example.com".to_string()));
/// assert_eq!(normalize_domain("www.example.com:8080"), Some("www.example.com".to_string()));
/// assert_eq!(normalize_domain("user@smtp.example.com"), Some("smtp.example.com".to_string()));
/// assert_eq!(normalize_domain("example.com."), Some("example.com".to_string()));
/// assert_eq!(normalize_domain(""), None);
/// assert_eq!(normalize_domain("   "), None);
/// ```
fn normalize_domain(raw: &str) -> Option<String> {
    // 去除首尾空白并转为小写
    let mut domain = raw.trim().to_ascii_lowercase();

    // 空字符串直接返回 None
    if domain.is_empty() {
        return None;
    }

    // 去除协议前缀（如 https://, http://, ftp:// 等）
    if let Some((_, rest)) = domain.split_once("://") {
        domain = rest.to_string();
    }

    // 去除路径、查询参数和片段标识符
    // 例如: "example.com/path?query#fragment" -> "example.com"
    domain = domain.split(['/', '?', '#']).next().unwrap_or_default().to_string();

    // 去除用户信息部分（如 "user:pass@example.com" 中的 "user:pass@"）
    if let Some((_, host)) = domain.rsplit_once('@') {
        domain = host.to_string();
    }

    // 去除端口号（如 "example.com:8080" 中的 ":8080"）
    if let Some((host, _port)) = domain.split_once(':') {
        domain = host.to_string();
    }

    // 去除末尾的点号（DNS 根域标识）
    domain = domain.trim_end_matches('.').to_string();

    // 最终检查是否为空
    if domain.is_empty() { None } else { Some(domain) }
}

/// 规范化并验证域名模式
///
/// 对域名模式进行格式验证和规范化处理。
/// 与 `normalize_domain` 不同，此函数用于处理模式定义而非实际 URL。
///
/// # 参数
///
/// - `raw`: 原始域名模式字符串
///
/// # 返回值
///
/// 成功时返回规范化后的模式字符串。
///
/// # 错误
///
/// 模式不符合规则时返回错误，错误信息包含具体原因。
///
/// # 验证规则
///
/// - 不能为空字符串
/// - 特殊值 `*` 表示匹配所有域名（通过验证）
/// - 不能以 `.` 开头或结尾
/// - 不能包含连续的 `..`
/// - 不能包含连续的 `**`
/// - 只能包含: `a-z`, `0-9`, `.`, `-`, `*`
/// - 所有标签（以 `.` 分隔的部分）不能为空
/// - `*.` 开头的通配符模式后必须有内容
///
/// # 示例
///
/// ```rust
/// assert_eq!(normalize_pattern("*.example.com")?, "*.example.com");
/// assert_eq!(normalize_pattern("  Example.COM  ")?, "example.com");  // 转小写
/// assert!(normalize_pattern("").is_err());
/// assert!(normalize_pattern(".example.com").is_err());
/// ```
fn normalize_pattern(raw: &str) -> Result<String> {
    // 去除首尾空白并转为小写
    let pattern = raw.trim().to_ascii_lowercase();

    // 不能为空
    if pattern.is_empty() {
        bail!("Domain pattern must not be empty");
    }

    // 特殊情况：单个 * 表示匹配所有域名
    if pattern == "*" {
        return Ok(pattern);
    }

    // 不能以 . 开头或结尾
    if pattern.starts_with('.') || pattern.ends_with('.') {
        bail!("Domain pattern '{raw}' must not start or end with '.'");
    }

    // 不能包含连续的点号
    if pattern.contains("..") {
        bail!("Domain pattern '{raw}' must not contain consecutive dots");
    }

    // 不能包含连续的星号
    if pattern.contains("**") {
        bail!("Domain pattern '{raw}' must not contain consecutive '*'");
    }

    // 验证字符集：只允许小写字母、数字、点号、连字符和星号
    if !pattern
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-' || c == '*')
    {
        bail!(
            "Domain pattern '{raw}' contains invalid characters; allowed: a-z, 0-9, '.', '-', '*'"
        );
    }

    // 检查每个标签（以 . 分隔）是否非空
    if pattern.split('.').any(|label| label.is_empty()) {
        bail!("Domain pattern '{raw}' contains an empty label");
    }

    // *. 开头的模式必须后跟实际域名（如 *.example.com）
    if pattern.starts_with("*.") && pattern.len() <= 2 {
        bail!("Domain pattern '{raw}' is incomplete");
    }

    Ok(pattern)
}

/// 检查域名是否匹配给定的模式
///
/// 判断给定的规范域名是否匹配指定的域名模式。
/// 支持通配符 `*` 匹配。
///
/// # 参数
///
/// - `pattern`: 域名模式（应已通过 `normalize_pattern` 规范化）
/// - `domain`: 待检查的域名（应已通过 `normalize_domain` 规范化）
///
/// # 返回值
///
/// - `true`: 域名匹配模式
/// - `false`: 域名不匹配模式
///
/// # 匹配规则
///
/// - `*` 匹配所有域名
/// - 不含 `*` 的模式要求精确匹配
/// - 含 `*` 的模式进行通配符匹配
///   - `*` 可以匹配零个或多个任意字符
///   - 例如 `*.example.com` 匹配 `www.example.com`、`api.example.com` 等
///
/// # 示例
///
/// ```rust
/// assert!(domain_matches_pattern("*", "any.domain.com"));
/// assert!(domain_matches_pattern("example.com", "example.com"));
/// assert!(!domain_matches_pattern("example.com", "other.com"));
/// assert!(domain_matches_pattern("*.example.com", "www.example.com"));
/// assert!(domain_matches_pattern("*.example.com", "api.example.com"));
/// assert!(!domain_matches_pattern("*.example.com", "example.com")); // 无子域名
/// ```
fn domain_matches_pattern(pattern: &str, domain: &str) -> bool {
    // 特殊情况：单个 * 匹配所有
    if pattern == "*" {
        return true;
    }

    // 不含通配符的情况：精确匹配
    if !pattern.contains('*') {
        return pattern == domain;
    }

    // 含通配符的情况：使用通配符匹配算法
    wildcard_match(pattern.as_bytes(), domain.as_bytes())
}

/// 通配符模式匹配算法
///
/// 实现支持 `*` 通配符的字符串匹配。
/// `*` 可以匹配零个或多个任意字符。
///
/// # 算法说明
///
/// 使用贪心回溯算法实现通配符匹配：
/// 1. 逐字符比较模式和值
/// 2. 遇到 `*` 时记录位置，尝试匹配零个字符
/// 3. 如果后续匹配失败，回溯到 `*` 位置，增加匹配字符数
/// 4. 重复直到成功或确定无法匹配
///
/// # 参数
///
/// - `pattern`: 模式字节序列（可包含 `*` 字符）
/// - `value`: 待匹配的字节序列
///
/// # 返回值
///
/// - `true`: 值匹配模式
/// - `false`: 值不匹配模式
///
/// # 性能
///
/// 时间复杂度：最坏情况 O(m*n)，其中 m、n 分别为模式和值的长度。
/// 空间复杂度：O(1)，仅使用常数额外空间。
///
/// # 示例
///
/// ```rust
/// assert!(wildcard_match(b"hello", b"hello"));
/// assert!(wildcard_match(b"he*o", b"hello"));
/// assert!(wildcard_match(b"*", b"anything"));
/// assert!(wildcard_match(b"a*b*c", b"abc"));
/// assert!(!wildcard_match(b"hello", b"world"));
/// ```
fn wildcard_match(pattern: &[u8], value: &[u8]) -> bool {
    // 模式指针：当前比较的模式位置
    let mut p = 0usize;
    // 值指针：当前比较的值位置
    let mut v = 0usize;
    // 最近一个星号的位置（用于回溯）
    let mut star_idx: Option<usize> = None;
    // 星号匹配的起始位置（在值中的位置）
    let mut match_idx = 0usize;

    // 主循环：遍历值的所有字符
    while v < value.len() {
        // 情况 1：当前字符匹配
        if p < pattern.len() && pattern[p] == value[v] {
            p += 1;
            v += 1;
            continue;
        }

        // 情况 2：遇到星号，记录位置以备回溯
        if p < pattern.len() && pattern[p] == b'*' {
            star_idx = Some(p);
            p += 1;
            match_idx = v;
            continue;
        }

        // 情况 3：字符不匹配，但有星号可以回溯
        if let Some(star) = star_idx {
            // 回溯：星号多匹配一个字符
            p = star + 1; // 从星号后的位置继续匹配
            match_idx += 1; // 星号匹配的字符数增加
            v = match_idx; // 从新的匹配位置继续
            continue;
        }

        // 情况 4：不匹配且无法回溯，匹配失败
        return false;
    }

    // 处理模式末尾可能的多余星号
    // 例如模式 "a*b*" 与值 "ab" 匹配时，末尾的 * 需要跳过
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }

    // 只有模式完全消费才算匹配成功
    p == pattern.len()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
