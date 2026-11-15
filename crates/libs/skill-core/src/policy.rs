//! 技能来源域名规范化与信任匹配；不执行交互或持久化。

/// 规范化域名条目
///
/// 将用户输入的域名字符串转换为标准化的域名格式，处理过程包括：
/// 1. 去除首尾空白字符
/// 2. 转换为小写
/// 3. 移除 `https://` 或 `http://` 协议前缀
/// 4. 移除路径、查询参数和片段（只保留主机部分）
/// 5. 移除通配符前缀（如 `*.`）和开头的点
/// 6. 移除端口号
///
/// # 参数
///
/// * `raw` - 原始的域名输入字符串，可能包含协议、路径、端口等
///
/// # 返回值
///
/// 返回规范化后的域名字符串。如果输入为空或仅包含空白字符，返回空字符串。
///
/// # 示例
///
/// ```ignore
/// assert_eq!(normalize_domain_entry("  HTTPS://GitHub.COM/foo/bar  "), "github.com");
/// assert_eq!(normalize_domain_entry("*.example.com"), "example.com");
/// assert_eq!(normalize_domain_entry("example.com:8080"), "example.com");
/// ```
fn normalize_domain_entry(raw: &str) -> String {
    let mut s = raw.trim().to_ascii_lowercase();
    if s.is_empty() {
        return s;
    }
    if let Some(rest) = s.strip_prefix("https://") {
        s = rest.to_string();
    } else if let Some(rest) = s.strip_prefix("http://") {
        s = rest.to_string();
    }
    s = s.split(&['/', '?', '#'][..]).next().unwrap_or("").trim().to_string();
    s = s.trim_start_matches("*.").trim_start_matches('.').to_string();
    if let Some((host, _port)) = s.split_once(':') {
        return host.to_string();
    }
    s
}

/// 规范化域名列表
///
/// 对域名列表进行批量规范化处理，包括：
/// 1. 对每个条目应用 `normalize_domain_entry` 规范化
/// 2. 过滤掉空条目
/// 3. 按字母顺序排序
/// 4. 去除重复项
///
/// # 参数
///
/// * `entries` - 可变引用，待规范化的域名列表
///
/// # 副作用
///
/// 直接修改传入的向量，替换为规范化后的结果。
pub fn normalize_domain_list(entries: &mut Vec<String>) {
    let mut normalized = entries
        .iter()
        .map(|entry| normalize_domain_entry(entry))
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    *entries = normalized;
}

/// 检查主机名是否匹配受信任的域名
///
/// 判断给定的主机名是否属于受信任域名或其子域名。支持通配符匹配，
/// 即如果信任域名为 `example.com`，则 `sub.example.com` 也会被视为匹配。
///
/// # 参数
///
/// * `host` - 待检查的主机名（URL 中的主机部分）
/// * `trusted_domain` - 受信任的域名模式
///
/// # 返回值
///
/// 如果主机名完全匹配受信任域名，或是受信任域名的子域名，返回 `true`；
/// 否则返回 `false`。如果任一参数规范化后为空，返回 `false`。
///
/// # 示例
///
/// ```ignore
/// assert!(host_matches_trusted_domain("github.com", "github.com"));
/// assert!(host_matches_trusted_domain("api.github.com", "github.com"));
/// assert!(!host_matches_trusted_domain("github.io", "github.com"));
/// ```
pub fn host_matches_trusted_domain(host: &str, trusted_domain: &str) -> bool {
    let host = normalize_domain_entry(host);
    let trusted = normalize_domain_entry(trusted_domain);
    if host.is_empty() || trusted.is_empty() {
        return false;
    }
    host == trusted || host.ends_with(&format!(".{trusted}"))
}

/// 检查主机名是否匹配任一域名列表中的条目
///
/// 遍历域名列表，检查主机名是否匹配其中任一域名。
///
/// # 参数
///
/// * `host` - 待检查的主机名
/// * `entries` - 域名列表
///
/// # 返回值
///
/// 如果主机名匹配列表中的任一域名（包括子域名匹配），返回 `true`；
/// 否则返回 `false`。
pub fn host_matches_any_domain(host: &str, entries: &[String]) -> bool {
    entries.iter().any(|entry| host_matches_trusted_domain(host, entry))
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod policy_tests;
