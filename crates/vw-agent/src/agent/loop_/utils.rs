//! 代理循环工具模块
//!
//! 本模块提供代理主循环中使用的实用工具函数和静态资源。
//! 主要功能包括：
//! - 敏感信息（凭证、密钥等）的检测和脱敏处理
//! - 工具输出中文件路径的提取
//!
//! # 安全性
//!
//! 本模块在安全敏感上下文中运行，用于防止敏感凭证在工具输出中意外泄露。

use regex::{Regex, RegexSet};
use std::sync::LazyLock;

/// 敏感键名模式集合
///
/// 包含常见敏感字段的正则表达式集合，用于快速判断字符串是否可能包含敏感信息。
/// 匹配的模式包括：
/// - token（令牌）
/// - api_key / api-key（API 密钥）
/// - password（密码）
/// - secret（秘密）
/// - user_key / user-key（用户密钥）
/// - bearer（Bearer 令牌）
/// - credential（凭证）
///
/// 所有模式均为不区分大小写匹配。
pub(crate) static SENSITIVE_KEY_PATTERNS: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new([
        r"(?i)token",
        r"(?i)api[_-]?key",
        r"(?i)password",
        r"(?i)secret",
        r"(?i)user[_-]?key",
        r"(?i)bearer",
        r"(?i)credential",
    ])
    .unwrap()
});

/// 敏感键值对正则表达式
///
/// 用于从文本中提取和匹配敏感凭证的键值对。
/// 支持多种格式：
/// - JSON 风格：`"token": "value"` 或 `token: "value"`
/// - 配置风格：`api_key=value` 或 `api_key="value"`
///
/// # 捕获组
///
/// - 第 1 组：键名（如 token、api_key 等）
/// - 第 2 组：双引号包裹的值（长度 >= 8）
/// - 第 3 组：单引号包裹的值（长度 >= 8）
/// - 第 4 组：无引号的字母数字值（长度 >= 8）
pub(crate) static SENSITIVE_KV_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(token|api[_-]?key|password|secret|user[_-]?key|bearer|credential)["']?\s*[:=]\s*(?:"([^"]{8,})"|'([^']{8,})'|([a-zA-Z0-9_\-\.]{8,}))"#).unwrap()
});

/// 工具文件路径正则表达式
///
/// 用于从工具输出中提取文件路径。
/// 匹配格式：`file_path: "/path/to/file"` 或 `path="/path/to/file"`
///
/// # 捕获组
///
/// - 第 1 组：文件路径
pub(crate) static TOOL_FILE_PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)(?:file_path|path)\s*[:=]\s*"([^"]+)""#).unwrap());

/// 从工具输出中脱敏凭证信息
///
/// 扫描输入文本，将匹配的敏感凭证替换为脱敏占位符，防止意外泄露。
/// 脱敏时保留值的前 4 个字符作为上下文，其余部分替换为 `*[REDACTED]`。
///
/// # 参数
///
/// - `input`: 需要脱敏的原始字符串
///
/// # 返回值
///
/// 返回脱敏后的字符串，所有敏感凭证值已被替换为 `prefix*[REDACTED]` 格式
///
/// # 示例
///
/// ```ignore
/// let output = r#"token: "abcdef1234567890""#;
/// let scrubbed = scrub_credentials(output);
/// // 结果: "token: "abcd*[REDACTED]""
/// ```
///
/// # 处理逻辑
///
/// 1. 使用 `SENSITIVE_KV_REGEX` 正则表达式匹配所有敏感键值对
/// 2. 对于每个匹配，提取键名和值
/// 3. 保留值的前 4 个字符，其余部分用 `*[REDACTED]` 替换
/// 4. 根据原始格式（使用 `:` 或 `=`，有无引号）重建脱敏后的字符串
pub(crate) fn scrub_credentials(input: &str) -> String {
    SENSITIVE_KV_REGEX
        .replace_all(input, |caps: &regex::Captures| {
            let full_match = &caps[0];
            let key = &caps[1];

            // 依次尝试三个捕获组：双引号值、单引号值、无引号值
            let val = caps.get(2).or(caps.get(3)).or(caps.get(4)).map(|m| m.as_str()).unwrap_or("");

            // 保留前 4 个字符作为上下文，避免完全丢失信息
            let prefix = val.chars().take(4).collect::<String>();

            // 根据原始格式重建脱敏后的字符串，保持格式一致性
            if full_match.contains(':') {
                // 冒号分隔符格式
                if full_match.contains('"') {
                    format!("\"{}\": \"{}*[REDACTED]\"", key, prefix)
                } else {
                    format!("{}: {}*[REDACTED]", key, prefix)
                }
            } else {
                // 等号分隔符格式
                if full_match.contains('"') {
                    format!("{}=\"{}*[REDACTED]\"", key, prefix)
                } else {
                    format!("{}={}*[REDACTED]", key, prefix)
                }
            }
        })
        .to_string()
}
