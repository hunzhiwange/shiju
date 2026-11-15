//! 通配符模式匹配工具模块
//!
//! 本模块提供基于通配符的模式匹配功能，支持 `*` 和 `?` 通配符。
//! 主要用于字符串模式匹配和结构化输入的匹配场景。
//!
//! # 功能特性
//!
//! - 支持标准通配符：`*`（匹配任意字符序列）和 `?`（匹配单个字符）
//! - 支持简单的结构化输入匹配（带头部和尾部的输入）
//! - 提供批量模式匹配，返回最具体（最长）的匹配结果

use regex::Regex;

/// 转义字符串中的正则表达式特殊字符
///
/// 将字符串中所有正则表达式元字符转义，使其作为字面量字符处理。
///
/// # 参数
///
/// * `s` - 需要转义的原始字符串
///
/// # 返回值
///
/// 返回转义后的字符串，所有正则元字符前都添加了反斜杠
///
/// # 示例
///
/// ```ignore
/// let escaped = escape_regex("file.txt");
/// assert_eq!(escaped, "file\\.txt");
/// ```
fn escape_regex(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        // 遇到正则表达式元字符时，添加反斜杠进行转义
        match ch {
            '.' | '+' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

/// 检查输入字符串是否匹配指定的通配符模式
///
/// 支持以下通配符：
/// - `*`：匹配任意数量的任意字符（包括零个字符）
/// - `?`：匹配恰好一个任意字符
///
/// 模式末尾的 `*` 会同时匹配带空格和不带空格的变体。
///
/// # 参数
///
/// * `input` - 待匹配的输入字符串
/// * `pattern` - 包含通配符的模式字符串
///
/// # 返回值
///
/// 如果输入匹配模式则返回 `true`，否则返回 `false`
///
/// # 示例
///
/// ```ignore
/// assert!(matches("hello world", "hello*"));
/// assert!(matches("test.txt", "test.?xt"));
/// assert!(!matches("hello", "world*"));
/// ```
pub fn matches(input: &str, pattern: &str) -> bool {
    // 先转义所有正则特殊字符，然后将通配符转换为正则表达式
    let mut escaped = escape_regex(pattern).replace('*', ".*").replace('?', ".");

    // 处理末尾的 " .*" 模式，使其同时匹配有无后续内容的情况
    // 例如 "hello *" 可以匹配 "hello" 和 "hello world"
    if escaped.ends_with(" .*") {
        escaped.truncate(escaped.len().saturating_sub(3));
        escaped.push_str("( .*)?");
    }

    // 构建正则表达式并进行匹配
    // (?s) 标志使 . 也能匹配换行符
    let re = Regex::new(&format!("(?s)^{}$", escaped)).ok();
    re.is_some_and(|re| re.is_match(input))
}

/// 在多个模式中查找匹配输入的第一个值
///
/// 按模式长度从短到长排序后依次匹配，返回最后一个匹配的值。
/// 这种策略确保返回最具体（最长模式）的匹配结果。
///
/// # 类型参数
///
/// * `T` - 关联值的类型，必须实现 `Clone` trait
///
/// # 参数
///
/// * `input` - 待匹配的输入字符串
/// * `patterns` - 模式与关联值的切片，格式为 `[(模式, 值), ...]`
///
/// # 返回值
///
/// 返回匹配的关联值（`Some`），如果没有匹配则返回 `None`
///
/// # 示例
///
/// ```ignore
/// let patterns = vec![
///     ("hello*".to_string(), 1),
///     ("hello world*".to_string(), 2),
/// ];
/// assert_eq!(all("hello world test", &patterns), Some(2));
/// assert_eq!(all("hello there", &patterns), Some(1));
/// assert_eq!(all("goodbye", &patterns), None);
/// ```
pub fn all<T: Clone>(input: &str, patterns: &[(String, T)]) -> Option<T> {
    // 将模式按长度排序，较短的模式优先匹配
    // 这样后匹配的更长模式会覆盖之前的结果
    let mut sorted: Vec<(&String, &T)> = patterns.iter().map(|(k, v)| (k, v)).collect();
    sorted.sort_by(|(a, _), (b, _)| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));

    let mut result: Option<&T> = None;
    for (pattern, value) in sorted {
        if matches(input, pattern) {
            result = Some(value);
        }
    }
    result.cloned()
}

/// 结构化输入
///
/// 表示一个具有头部和尾部的结构化输入，用于更复杂的模式匹配场景。
/// 头部通常是命令或关键词，尾部是参数或附加信息。
///
/// # 字段
///
/// * `head` - 输入的头部字符串（如命令名）
/// * `tail` - 输入的尾部字符串向量（如参数列表）
pub struct StructuredInput {
    /// 输入的头部字符串，通常是主要标识符或命令
    pub head: String,
    /// 输入的尾部字符串列表，通常是参数或附加数据
    pub tail: Vec<String>,
}

/// 在多个模式中查找匹配结构化输入的第一个值
///
/// 模式使用空格分隔，第一部分匹配输入的头部，其余部分匹配尾部序列。
/// 尾部匹配支持 `*` 通配符跳过任意数量的尾部元素。
///
/// # 类型参数
///
/// * `T` - 关联值的类型，必须实现 `Clone` trait
///
/// # 参数
///
/// * `input` - 结构化输入，包含头部和尾部
/// * `patterns` - 模式与关联值的切片，模式以空格分隔
///
/// # 返回值
///
/// 返回匹配的关联值（`Some`），如果没有匹配则返回 `None`
///
/// # 示例
///
/// ```ignore
/// let input = StructuredInput {
///     head: "cmd".to_string(),
///     tail: vec!["arg1".to_string(), "arg2".to_string()],
/// };
/// let patterns = vec![
///     ("cmd *".to_string(), 1),
///     ("cmd arg1 *".to_string(), 2),
/// ];
/// assert_eq!(all_structured(&input, &patterns), Some(2));
/// ```
pub fn all_structured<T: Clone>(input: &StructuredInput, patterns: &[(String, T)]) -> Option<T> {
    // 按模式长度排序，优先匹配较短模式
    let mut sorted: Vec<(&String, &T)> = patterns.iter().map(|(k, v)| (k, v)).collect();
    sorted.sort_by(|(a, _), (b, _)| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));

    let mut result: Option<&T> = None;
    for (pattern, value) in sorted {
        // 将模式按空格分割为多个部分
        let parts: Vec<&str> = pattern.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        // 首先检查头部是否匹配
        if !matches(&input.head, parts[0]) {
            continue;
        }

        // 如果只有头部模式，或者尾部序列也匹配，则记录结果
        if parts.len() == 1 || match_sequence(&input.tail, &parts[1..]) {
            result = Some(value);
        }
    }
    result.cloned()
}

/// 递归匹配尾部序列与模式序列
///
/// 支持使用 `*` 通配符跳过任意数量的尾部元素。
/// 递归尝试在每个可能的位置匹配剩余模式。
///
/// # 参数
///
/// * `items` - 尾部字符串切片
/// * `patterns` - 模式字符串切片
///
/// # 返回值
///
/// 如果尾部序列能够匹配模式序列则返回 `true`，否则返回 `false`
fn match_sequence(items: &[String], patterns: &[&str]) -> bool {
    // 如果没有更多模式需要匹配，则匹配成功
    if patterns.is_empty() {
        return true;
    }

    let (first, rest) = patterns.split_first().unwrap();

    // 如果当前模式是 `*`，跳过它并继续匹配剩余模式
    if *first == "*" {
        return match_sequence(items, rest);
    }

    // 尝试在尾部的每个位置开始匹配当前模式
    // 找到第一个匹配的位置后，递归匹配剩余模式
    for (i, item) in items.iter().enumerate() {
        if matches(item, first) && match_sequence(&items[i + 1..], rest) {
            return true;
        }
    }
    false
}
