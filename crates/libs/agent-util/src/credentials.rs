//! Credential redaction shared by model recovery and tool output.
use regex::Regex;
use std::sync::LazyLock;

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
pub static SENSITIVE_KV_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(token|api[_-]?key|password|secret|user[_-]?key|bearer|credential)["']?\s*[:=]\s*(?:"([^"]{8,})"|'([^']{8,})'|([a-zA-Z0-9_\-\.]{8,}))"#).unwrap()
});

pub fn scrub_credentials(input: &str) -> String {
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
