//! Shell 重定向处理模块
//!
//! 本模块提供了 Shell 命令重定向的安全处理功能，用于识别和移除安全的重定向操作。
//! 主要用于安全策略中，确保代理在执行 Shell 命令时不会意外阻止常用的安全重定向模式。
//!
//! # 核心功能
//!
//! - **流合并重定向**：支持 `2>&1`、`1>&2` 等文件描述符合并语法
//! - **/dev/null 重定向**：支持 `>/dev/null`、`2>/dev/null`、`&>/dev/null` 等静默输出模式
//! - **重定向目标提取**：从重定向 token 中提取目标路径
//! - **短选项值提取**：从 `-opt value` 形式的 token 中提取值部分
//!
//! # 安全考量
//!
//! 本模块仅支持移除受限的安全重定向模式。复杂的重定向、管道组合或
//! 指向任意文件路径的重定向不会被自动移除，以确保安全边界。

use super::types::QuoteState;

/// 判断给定字符是否为 token 边界字符
///
/// Token 边界字符用于分隔 Shell 命令中的不同语法单元。
/// 包括空白字符和常见的 Shell 操作符。
///
/// # 参数
///
/// - `ch`: 待检查的字符
///
/// # 返回值
///
/// 如果字符是边界字符则返回 `true`，否则返回 `false`
///
/// # 边界字符列表
///
/// - 空白字符（空格、制表符等）
/// - `;` - 命令分隔符
/// - `\n` - 换行符
/// - `|` - 管道符
/// - `&` - 后台运行符
/// - `)` 和 `(` - 子 shell 分组符
fn is_token_boundary_char(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, ';' | '\n' | '|' | '&' | ')' | '(')
}

/// 检查字符数组从指定位置开始是否匹配给定的字面值
///
/// # 参数
///
/// - `chars`: 字符数组切片
/// - `start`: 开始检查的索引位置
/// - `literal`: 要匹配的字面值字符串
///
/// # 返回值
///
/// 如果从 `start` 位置开始的字符序列与 `literal` 完全匹配，返回 `true`
///
/// # 示例
///
/// ```ignore
/// let chars: Vec<char> = "hello world".chars().collect();
/// assert!(starts_with_literal(&chars, 0, "hello"));  // true
/// assert!(!starts_with_literal(&chars, 0, "world")); // false
/// assert!(starts_with_literal(&chars, 6, "world"));  // true
/// ```
fn starts_with_literal(chars: &[char], start: usize, literal: &str) -> bool {
    let literal_chars: Vec<char> = literal.chars().collect();
    chars.get(start..start + literal_chars.len()).is_some_and(|slice| slice == literal_chars)
}

/// 消费流合并重定向语法
///
/// 解析并消费形如 `2>&1`、`1>&2` 的文件描述符合并重定向。
/// 允许在文件描述符和操作符之间存在空白字符。
///
/// # 参数
///
/// - `chars`: 命令字符串的字符数组
/// - `start`: 开始解析的索引位置
///
/// # 返回值
///
/// 成功时返回 `Some(consumed)`，其中 `consumed` 是消费的字符数；
/// 如果当前位置不是有效的流合并重定向，返回 `None`
///
/// # 支持的语法
///
/// - `2>&1` - 将标准错误重定向到标准输出
/// - `1>&2` - 将标准输出重定向到标准错误
/// - `2> &1` - 带空白的变体
/// - `10>&2` - 多位数文件描述符
///
/// # 不支持的语法
///
/// - `>&2` - 缺少源文件描述符
/// - `2>&` - 缺少目标文件描述符
fn consume_stream_merge_redirect(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;

    // 消费源文件描述符（一个或多个数字）
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }

    // 检查是否紧跟 '>' 操作符
    if i >= chars.len() || chars[i] != '>' {
        return None;
    }
    i += 1;

    // 跳过可选的空白字符
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }

    // 检查是否紧跟 '&' 符号（表示文件描述符引用）
    if i >= chars.len() || chars[i] != '&' {
        return None;
    }
    i += 1;

    // 跳过可选的空白字符
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }

    // 消费目标文件描述符（一个或多个数字）
    let fd_start = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }

    // 确保至少有一个目标文件描述符数字
    if i == fd_start {
        return None;
    }

    Some(i - start)
}

/// 消费 /dev/null 重定向语法
///
/// 解析并消费将输出重定向到 `/dev/null` 的各种语法形式。
/// 这是一种常见的静默命令输出的方式。
///
/// # 参数
///
/// - `chars`: 命令字符串的字符数组
/// - `start`: 开始解析的索引位置
///
/// # 返回值
///
/// 成功时返回 `Some(consumed)`，其中 `consumed` 是消费的字符数；
/// 如果当前位置不是有效的 /dev/null 重定向，返回 `None`
///
/// # 支持的语法
///
/// - `>/dev/null` - 重定向标准输出
/// - `2>/dev/null` - 重定向标准错误
/// - `&>/dev/null` - 重定向所有输出
/// - `>>/dev/null` - 追加模式
/// - `2>>/dev/null` - 追加标准错误
/// - `<` 相关的输入重定向（虽然对 /dev/null 较少见）
///
/// # 验证规则
///
/// - `/dev/null` 后必须跟随 token 边界字符（空白、分号等）
/// - 防止匹配 `/dev/null_something` 等无效路径
fn consume_dev_null_redirect(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;

    // 处理 `&>` 开头的重定向（将所有输出重定向）
    if chars[i] == '&' {
        i += 1;
        if i >= chars.len() || chars[i] != '>' {
            return None;
        }
        i += 1;
    } else {
        // 处理数字前缀的重定向（如 2>、1>>）
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }

        // 检查重定向操作符（> 或 <）
        if i >= chars.len() || !matches!(chars[i], '>' | '<') {
            return None;
        }
        let op = chars[i];
        i += 1;

        // 处理追加重定向 `>>`
        if op == '>' && i < chars.len() && chars[i] == '>' {
            i += 1;
        }

        // 处理文件描述符合并 `>&`
        if op == '>' && i < chars.len() && chars[i] == '&' {
            i += 1;
        }
    }

    // 跳过重定向操作符后的空白字符
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }

    // 检查是否指向 /dev/null
    if !starts_with_literal(chars, i, "/dev/null") {
        return None;
    }
    i += "/dev/null".chars().count();

    // 确保 /dev/null 后面是 token 边界，防止匹配 /dev/null_other 等
    if i < chars.len() && !is_token_boundary_char(chars[i]) {
        return None;
    }

    Some(i - start)
}

/// 从 Shell 命令中移除支持的重定向
///
/// 解析 Shell 命令字符串，移除安全的重定向操作（流合并和 /dev/null 重定向），
/// 同时保留命令的其他部分不变。正确处理引号和转义字符。
///
/// # 参数
///
/// - `command`: 原始 Shell 命令字符串
///
/// # 返回值
///
/// 返回移除了支持的重定向后的命令字符串
///
/// # 示例
///
/// ```ignore
/// // 移除流合并重定向
/// assert_eq!(
///     strip_supported_redirects("cmd 2>&1"),
///     "cmd "
/// );
///
/// // 移除 /dev/null 重定向
/// assert_eq!(
///     strip_supported_redirects("cmd >/dev/null"),
///     "cmd "
/// );
///
/// // 保留引号内的内容
/// assert_eq!(
///     strip_supported_redirects("echo \"2>&1\""),
///     "echo \"2>&1\""
/// );
/// ```
///
/// # 处理规则
///
/// 1. **引号保护**：单引号和双引号内的内容不会被处理
/// 2. **转义保护**：反斜杠转义的字符会被保留
/// 3. **管道保留**：`|&` 管道语法会被保留（不是重定向）
/// 4. **优先级**：先尝试匹配流合并重定向，再尝试 /dev/null 重定向
///
/// # 安全说明
///
/// 此函数仅移除预定义的安全重定向模式，不会移除任意文件路径的重定向。
/// 这确保了代理无法通过重定向将敏感数据写入任意位置。
pub fn strip_supported_redirects(command: &str) -> String {
    let chars: Vec<char> = command.chars().collect();
    let mut out = String::with_capacity(command.len());
    let mut quote = QuoteState::None;
    let mut escaped = false;
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        match quote {
            // 单引号状态：不处理转义，所有字符原样输出
            QuoteState::Single => {
                if ch == '\'' {
                    quote = QuoteState::None;
                }
                out.push(ch);
                i += 1;
            }
            // 双引号状态：支持反斜杠转义
            QuoteState::Double => {
                if escaped {
                    escaped = false;
                    out.push(ch);
                    i += 1;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    out.push(ch);
                    i += 1;
                    continue;
                }
                if ch == '"' {
                    quote = QuoteState::None;
                }
                out.push(ch);
                i += 1;
            }
            // 无引号状态：检查重定向和引号开始
            QuoteState::None => {
                // 处理转义字符
                if escaped {
                    escaped = false;
                    out.push(ch);
                    i += 1;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    out.push(ch);
                    i += 1;
                    continue;
                }
                // 检测单引号开始
                if ch == '\'' {
                    quote = QuoteState::Single;
                    out.push(ch);
                    i += 1;
                    continue;
                }
                // 检测双引号开始
                if ch == '"' {
                    quote = QuoteState::Double;
                    out.push(ch);
                    i += 1;
                    continue;
                }

                // 保留 |& 管道语法（将标准错误也管道到下一个命令）
                // 这不是重定向，不应被移除
                if ch == '|' && chars.get(i + 1).is_some_and(|next| *next == '&') {
                    out.push('|');
                    i += 2;
                    continue;
                }

                // 尝试匹配并跳过支持的重定向
                // 优先尝试流合并重定向（如 2>&1）
                // 然后尝试 /dev/null 重定向
                if let Some(consumed) = consume_stream_merge_redirect(&chars, i)
                    .or_else(|| consume_dev_null_redirect(&chars, i))
                {
                    i += consumed;
                    continue;
                }

                // 默认情况：保留字符
                out.push(ch);
                i += 1;
            }
        }
    }

    out.trim_end().to_string()
}

/// 从短选项 token 中提取附加的选项值
///
/// 解析形如 `-opt value` 或 `-opt=value` 的短选项 token，
/// 提取选项后面的值部分。用于检测重定向目标是否伪装成选项值。
///
/// # 参数
///
/// - `token`: 待解析的选项 token
///
/// # 返回值
///
/// - `Some(value)`: 成功提取到选项值
/// - `None`: token 不是有效的短选项形式，或没有附加值
///
/// # 示例
///
/// ```ignore
/// assert_eq!(attached_short_option_value("-f file.txt"), Some("file.txt"));
/// assert_eq!(attached_short_option_value("-ofile.txt"), Some("file.txt"));
/// assert_eq!(attached_short_option_value("-o=file.txt"), Some("file.txt"));
/// assert_eq!(attached_short_option_value("-o"), None);
/// assert_eq!(attached_short_option_value("--option"), None);
/// assert_eq!(attached_short_option_value("not-option"), None);
/// ```
///
/// # 匹配规则
///
/// 1. token 必须以单个 `-` 开头（排除 `--` 长选项）
/// 2. `-` 后必须至少有两个字符（选项名 + 值）
/// 3. 值部分可以以 `=` 开头或直接跟在选项名后
pub fn attached_short_option_value(token: &str) -> Option<&str> {
    // 移除前导 `-`，如果不是以 `-` 开头则返回 None
    let body = token.strip_prefix('-')?;

    // 排除长选项（以 `-` 开头）和过短的选项（少于2个字符）
    if body.starts_with('-') || body.len() < 2 {
        return None;
    }

    // 提取选项名之后的部分，跳过可选的 `=` 前缀和空白
    let value = body[1..].trim_start_matches('=').trim();

    // 如果值为空则返回 None，否则返回值
    if value.is_empty() { None } else { Some(value) }
}

/// 从重定向 token 中提取目标路径
///
/// 解析重定向操作符（`<` 或 `>`），提取重定向的目标路径或文件描述符。
///
/// # 参数
///
/// - `token`: 包含重定向操作符的 token
///
/// # 返回值
///
/// - `Some(target)`: 成功提取到重定向目标
/// - `None`: token 不包含有效的重定向操作符或没有目标
///
/// # 示例
///
/// ```ignore
/// assert_eq!(redirection_target(">file.txt"), Some("file.txt"));
/// assert_eq!(redirection_target(">>file.txt"), Some("file.txt"));
/// assert_eq!(redirection_target("2>&1"), Some("1"));
/// assert_eq!(redirection_target("<input.txt"), Some("input.txt"));
/// assert_eq!(redirection_target("no-redirect"), None);
/// ```
///
/// # 提取规则
///
/// 1. 查找第一个 `<` 或 `>` 字符
/// 2. 跳过后续的重定向操作符（如 `>>` 中的第二个 `>`）
/// 3. 跳过 `&` 符号（用于文件描述符引用）
/// 4. 跳过数字（用于文件描述符合并语法中的目标描述符）
/// 5. 返回剩余部分去除空白后的结果
///
/// # 注意事项
///
/// 此函数仅执行语法提取，不验证目标路径的合法性或安全性。
/// 调用者应自行对提取的目标进行安全检查。
pub fn redirection_target(token: &str) -> Option<&str> {
    // 查找重定向操作符的位置
    let marker_idx = token.find(['<', '>'])?;

    // 从操作符之后开始处理
    let mut rest = &token[marker_idx + 1..];

    // 跳过重复的重定向操作符（如 >>）
    rest = rest.trim_start_matches(['<', '>']);

    // 跳过文件描述符引用符号
    rest = rest.trim_start_matches('&');

    // 跳过数字（用于 2>&1 等语法中的目标描述符）
    rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());

    // 去除前导和尾随空白
    let trimmed = rest.trim();

    // 如果结果为空则返回 None，否则返回提取的目标
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

#[cfg(test)]
#[path = "shell_redirect_tests.rs"]
mod shell_redirect_tests;
