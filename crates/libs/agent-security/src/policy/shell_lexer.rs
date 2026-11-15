//! Shell 命令词法分析器
//!
//! 本模块提供用于解析和分析 Shell 命令字符串的工具函数。
//! 主要用于安全策略中识别潜在危险的命令结构，如命令分隔符、
//! 变量展开、后台执行等。
//!
//! # 主要功能
//!
//! - 跳过环境变量赋值部分，提取实际执行的命令
//! - 按命令分隔符分割命令字符串（支持引号和转义）
//! - 检测命令中的特定字符或结构（考虑引号和转义的影响）
//! - 处理 Shell 变量展开的识别
//!
//! # 引号处理
//!
//! 所有函数都正确处理单引号和双引号内的内容：
//! - 单引号内的所有字符均为字面量，包括反斜杠
//! - 双引号内支持反斜杠转义

use super::types::QuoteState;

/// 跳过命令字符串开头的环境变量赋值部分
///
/// Shell 命令可以在命令名前设置环境变量，如 `VAR=value cmd arg`。
/// 此函数跳过这些赋值，返回实际命令开始的位置。
///
/// # 参数
///
/// - `s`: 待处理的命令字符串
///
/// # 返回值
///
/// 返回跳过环境变量赋值后的字符串切片。
/// 如果没有环境变量赋值，返回原字符串。
///
/// # 环境变量赋值规则
///
/// 有效的环境变量赋值必须满足：
/// - 包含等号 `=`
/// - 变量名以字母或下划线开头
///
/// # 示例
///
/// ```ignore
/// let cmd = "FOO=bar BAZ=qux echo hello";
/// let result = skip_env_assignments(cmd);
/// assert_eq!(result, "echo hello");
///
/// let cmd = "echo hello";
/// let result = skip_env_assignments(cmd);
/// assert_eq!(result, "echo hello");
/// ```
pub fn skip_env_assignments(s: &str) -> &str {
    let mut rest = s;
    loop {
        // 获取下一个空白分隔的单词
        let Some(word) = rest.split_whitespace().next() else {
            return rest;
        };
        // 检查是否为有效的环境变量赋值：
        // 1. 包含等号
        // 2. 第一个字符是字母或下划线（符合 Shell 变量命名规则）
        if word.contains('=')
            && word.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        {
            // 跳过这个环境变量赋值，继续处理剩余部分
            rest = rest[word.len()..].trim_start();
        } else {
            // 不是环境变量赋值，返回当前位置
            return rest;
        }
    }
}

/// 按命令分隔符分割命令字符串为多个段
///
/// 将命令字符串按 Shell 命令分隔符（`;`、`\n`、`|`、`||`、`&&`）分割成多个段。
/// 正确处理引号和转义字符，不会在引号内或转义后的分隔符处分割。
///
/// # 参数
///
/// - `command`: 待分割的命令字符串
///
/// # 返回值
///
/// 返回分割后的命令段向量，每个段都是去除前后空白后的字符串。
/// 空白段会被过滤掉。
///
/// # 分隔符说明
///
/// - `;` - 命令顺序执行分隔符
/// - `\n` - 换行符（也是命令分隔符）
/// - `|` - 管道符
/// - `||` - 逻辑或（失败时执行下一个命令）
/// - `&&` - 逻辑与（成功时执行下一个命令）
///
/// # 引号处理
///
/// - 单引号内的分隔符被视为字面量
/// - 双引号内的分隔符被视为字面量
/// - 转义的分隔符被视为字面量
///
/// # 示例
///
/// ```ignore
/// let cmd = "echo hello; ls -la";
/// let segments = split_unquoted_segments(cmd);
/// assert_eq!(segments, vec!["echo hello", "ls -la"]);
///
/// let cmd = "echo 'hello; world'";
/// let segments = split_unquoted_segments(cmd);
/// assert_eq!(segments, vec!["echo 'hello; world'"]);
/// ```
pub fn split_unquoted_segments(command: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut quote = QuoteState::None;
    let mut escaped = false;
    let mut chars = command.chars().peekable();

    // 内部辅助函数：将当前累积的字符推入结果段
    //
    // 去除当前字符串的前后空白，如果非空则加入结果向量，然后清空当前字符串。
    let push_segment = |segments: &mut Vec<String>, current: &mut String| {
        let trimmed = current.trim();
        if !trimmed.is_empty() {
            segments.push(trimmed.to_string());
        }
        current.clear();
    };

    // 遍历命令字符串的每个字符
    while let Some(ch) = chars.next() {
        match quote {
            // 单引号状态：所有字符都是字面量，直到遇到结束的单引号
            QuoteState::Single => {
                if ch == '\'' {
                    quote = QuoteState::None;
                }
                current.push(ch);
            }
            // 双引号状态：支持反斜杠转义
            QuoteState::Double => {
                if escaped {
                    // 前一个字符是反斜杠，当前字符被转义
                    escaped = false;
                    current.push(ch);
                    continue;
                }
                if ch == '\\' {
                    // 遇到反斜杠，标记下一个字符为转义
                    escaped = true;
                    current.push(ch);
                    continue;
                }
                if ch == '"' {
                    // 双引号结束
                    quote = QuoteState::None;
                }
                current.push(ch);
            }
            // 非引号状态：检查分隔符和引号开始
            QuoteState::None => {
                if escaped {
                    // 前一个字符是反斜杠，当前字符被转义
                    escaped = false;
                    current.push(ch);
                    continue;
                }
                if ch == '\\' {
                    // 遇到反斜杠，标记下一个字符为转义
                    escaped = true;
                    current.push(ch);
                    continue;
                }

                match ch {
                    '\'' => {
                        // 开始单引号
                        quote = QuoteState::Single;
                        current.push(ch);
                    }
                    '"' => {
                        // 开始双引号
                        quote = QuoteState::Double;
                        current.push(ch);
                    }
                    ';' | '\n' => {
                        // 遇到分号或换行符，分割命令段
                        push_segment(&mut segments, &mut current);
                    }
                    '|' => {
                        // 管道符：检查是否为 || (逻辑或)
                        // peek 检查下一个字符但不消费它
                        if chars.next_if_eq(&'|').is_some() {}
                        push_segment(&mut segments, &mut current);
                    }
                    '&' => {
                        // 检查是否为 && (逻辑与)
                        if chars.next_if_eq(&'&').is_some() {
                            // 是 &&，分割命令段
                            push_segment(&mut segments, &mut current);
                        } else {
                            // 单个 &，作为后台运行符保留在当前段中
                            current.push(ch);
                        }
                    }
                    _ => current.push(ch),
                }
            }
        }
    }

    // 处理最后一个段（如果非空）
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        segments.push(trimmed.to_string());
    }

    segments
}

/// 检查命令中是否包含非引号内的单个 & 符号（后台运行符）
///
/// 单个 `&` 在 Shell 中表示后台运行命令，这可能存在安全风险。
/// 此函数检测是否存在未被引号包裹或转义的单个 `&` 符号。
///
/// # 参数
///
/// - `command`: 待检查的命令字符串
///
/// # 返回值
///
/// - `true`: 命令中包含非引号内且未被转义的单个 `&` 符号
/// - `false`: 命令中不包含这样的 `&` 符号
///
/// # 注意事项
///
/// - `&&` (逻辑与) 不会被检测为单个 `&`
/// - 引号内的 `&` 被视为字面量，不会触发检测
/// - 转义的 `\&` 不会触发检测
///
/// # 示例
///
/// ```ignore
/// assert_eq!(contains_unquoted_single_ampersand("cmd &"), true);
/// assert_eq!(contains_unquoted_single_ampersand("cmd && other"), false);
/// assert_eq!(contains_unquoted_single_ampersand("echo '&'"), false);
/// assert_eq!(contains_unquoted_single_ampersand("echo \\&"), false);
/// ```
pub fn contains_unquoted_single_ampersand(command: &str) -> bool {
    let mut quote = QuoteState::None;
    let mut escaped = false;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        match quote {
            // 单引号状态：忽略所有内容直到遇到结束引号
            QuoteState::Single => {
                if ch == '\'' {
                    quote = QuoteState::None;
                }
            }
            // 双引号状态：支持转义
            QuoteState::Double => {
                if escaped {
                    escaped = false;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    continue;
                }
                if ch == '"' {
                    quote = QuoteState::None;
                }
            }
            // 非引号状态：检查 & 符号和引号开始
            QuoteState::None => {
                if escaped {
                    escaped = false;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    continue;
                }
                match ch {
                    '\'' => quote = QuoteState::Single,
                    '"' => quote = QuoteState::Double,
                    '&' => {
                        // 检查下一个字符是否也是 &
                        // 如果不是，说明是单个 &（后台运行符）
                        if chars.next_if_eq(&'&').is_none() {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    false
}

/// 检查命令中是否包含非引号内的指定字符
///
/// 这是一个通用的字符检测函数，可以检测任何特定字符是否出现在
/// 非引号包裹且未被转义的位置。
///
/// # 参数
///
/// - `command`: 待检查的命令字符串
/// - `target`: 要查找的目标字符
///
/// # 返回值
///
/// - `true`: 命令中包含非引号内且未被转义的目标字符
/// - `false`: 命令中不包含这样的目标字符
///
/// # 引号处理
///
/// - 单引号内的目标字符被视为字面量
/// - 双引号内的目标字符被视为字面量
/// - 转义的目标字符不会触发检测
///
/// # 示例
///
/// ```ignore
/// assert_eq!(contains_unquoted_char("cmd; ls", ';'), true);
/// assert_eq!(contains_unquoted_char("echo ';'", ';'), false);
/// assert_eq!(contains_unquoted_char("echo \\;", ';'), false);
/// ```
pub fn contains_unquoted_char(command: &str, target: char) -> bool {
    let mut quote = QuoteState::None;
    let mut escaped = false;

    for ch in command.chars() {
        match quote {
            // 单引号状态：忽略所有内容直到遇到结束引号
            QuoteState::Single => {
                if ch == '\'' {
                    quote = QuoteState::None;
                }
            }
            // 双引号状态：支持转义
            QuoteState::Double => {
                if escaped {
                    escaped = false;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    continue;
                }
                if ch == '"' {
                    quote = QuoteState::None;
                }
            }
            // 非引号状态：检查目标字符和引号开始
            QuoteState::None => {
                if escaped {
                    escaped = false;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    continue;
                }
                match ch {
                    '\'' => quote = QuoteState::Single,
                    '"' => quote = QuoteState::Double,
                    _ if ch == target => return true,
                    _ => {}
                }
            }
        }
    }

    false
}

/// 检查命令中是否包含非引号内的 Shell 变量展开
///
/// 检测命令中是否存在 Shell 变量展开语法（以 `$` 开头的各种形式）。
/// 这些展开可能导致命令注入或不可预期的行为。
///
/// # 参数
///
/// - `command`: 待检查的命令字符串
///
/// # 返回值
///
/// - `true`: 命令中包含非引号内且未被转义的变量展开
/// - `false`: 命令中不包含这样的变量展开
///
/// # 支持的变量展开形式
///
/// - `$VAR` - 普通变量引用
/// - `${VAR}` - 大括号包裹的变量引用
/// - `$(cmd)` - 命令替换
/// - `$((expr))` - 算术展开（由 `$(` 开始）
/// - `$?` - 上一个命令的退出状态
/// - `$$` - 当前进程 ID
/// - `$!` - 最近的后台任务进程 ID
/// - `$*` - 所有位置参数
/// - `$@` - 所有位置参数（数组形式）
/// - `$#` - 位置参数个数
/// - `$-` - 当前 Shell 选项
/// - `$0`-`$9` - 位置参数（由字母数字匹配覆盖）
///
/// # 引号处理
///
/// - 单引号内的 `$` 被视为字面量，不会触发检测
/// - 双引号内的 `$` 会触发检测（Shell 在双引号内仍会展开变量）
/// - 转义的 `\$` 不会触发检测
///
/// # 示例
///
/// ```ignore
/// assert_eq!(contains_unquoted_shell_variable_expansion("echo $HOME"), true);
/// assert_eq!(contains_unquoted_shell_variable_expansion("echo '$HOME'"), false);
/// assert_eq!(contains_unquoted_shell_variable_expansion("echo \"$HOME\""), true);
/// assert_eq!(contains_unquoted_shell_variable_expansion("echo \\$HOME"), false);
/// assert_eq!(contains_unquoted_shell_variable_expansion("cmd $(whoami)"), true);
/// ```
pub fn contains_unquoted_shell_variable_expansion(command: &str) -> bool {
    let mut quote = QuoteState::None;
    let mut escaped = false;
    let chars: Vec<char> = command.chars().collect();

    for i in 0..chars.len() {
        let ch = chars[i];

        // 首先处理引号状态转换
        match quote {
            // 单引号状态：所有内容都是字面量，跳过变量检测
            QuoteState::Single => {
                if ch == '\'' {
                    quote = QuoteState::None;
                }
                continue;
            }
            // 双引号状态：支持转义，但变量展开仍然有效
            QuoteState::Double => {
                if escaped {
                    escaped = false;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    continue;
                }
                if ch == '"' {
                    quote = QuoteState::None;
                    continue;
                }
            }
            // 非引号状态：检查引号开始和转义
            QuoteState::None => {
                if escaped {
                    escaped = false;
                    continue;
                }
                if ch == '\\' {
                    escaped = true;
                    continue;
                }
                if ch == '\'' {
                    quote = QuoteState::Single;
                    continue;
                }
                if ch == '"' {
                    quote = QuoteState::Double;
                    continue;
                }
            }
        }

        // 如果当前字符不是 $，跳过变量检测
        if ch != '$' {
            continue;
        }

        // 检查 $ 后面的字符以确定是否为有效的变量展开
        let Some(next) = chars.get(i + 1).copied() else {
            continue;
        };

        // 检查是否符合 Shell 变量展开的各种形式
        if next.is_ascii_alphanumeric()
            || matches!(next, '_' | '{' | '(' | '#' | '?' | '!' | '$' | '*' | '@' | '-')
        {
            return true;
        }
    }

    false
}

/// 去除字符串两端包裹的引号
///
/// 如果字符串两端被单引号或双引号包裹，则去除这些引号。
/// 只去除两端完全匹配的引号，不会去除内部的引号。
///
/// # 参数
///
/// - `token`: 待处理的字符串
///
/// # 返回值
///
/// 返回去除两端引号后的字符串切片。
/// 如果两端没有被引号包裹，返回原字符串。
///
/// # 注意事项
///
/// - 同时去除两端可能存在的混合引号（如 `"test'` 会变成 `test`）
/// - 不会处理转义的引号
/// - 使用 `trim_matches` 实现，会去除所有两端出现的引号字符
///
/// # 示例
///
/// ```ignore
/// assert_eq!(strip_wrapping_quotes("\"hello\""), "hello");
/// assert_eq!(strip_wrapping_quotes("'hello'"), "hello");
/// assert_eq!(strip_wrapping_quotes("hello"), "hello");
/// assert_eq!(strip_wrapping_quotes("\"hello'"), "hello");
/// ```
pub fn strip_wrapping_quotes(token: &str) -> &str {
    token.trim_matches(|c| c == '"' || c == '\'')
}

#[cfg(test)]
#[path = "shell_lexer_tests.rs"]
mod shell_lexer_tests;
