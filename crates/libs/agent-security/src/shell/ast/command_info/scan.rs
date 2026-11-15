//! Shell 命令信息的保守字符串扫描器。
//!
//! 当 AST 特性不可用或 AST 无法提取命令时，本模块以手写扫描方式识别命令名、参数、
//! 管道、复合操作符、重定向以及常见 shell 展开。扫描器刻意保持有限语法支持：
//! 遇到不完整引号或无法闭合的替换表达式时返回 `None`，由调用方处理回退。

use super::super::nodes::{CommandInfo, CompoundOp, PipeSegment, Redirect, RedirectKind};
use super::MAX_SUBCOMMAND_DEPTH;

#[derive(Debug, Clone, Default)]
struct Token {
    text: String,
    has_command_substitution: bool,
    has_process_substitution: bool,
    has_glob: bool,
    has_variable_expansion: bool,
    subcommands: Vec<CommandInfo>,
}

pub(super) fn parse_command_info(command: &str, depth: usize) -> Option<CommandInfo> {
    if command.is_empty() || depth > MAX_SUBCOMMAND_DEPTH {
        // 字符串扫描会递归进入命令替换；深度限制让不可信输入不会造成无界解析。
        return None;
    }

    if let Some(inner) = subshell_inner(command) {
        let mut info = parse_command_info(inner.trim(), depth + 1)?;
        info.compound_operator = Some(CompoundOp::Subshell);
        return Some(info);
    }

    if let Some((left, op, _right)) = split_top_level_compound(command) {
        let mut info = parse_pipeline_or_simple(left.trim(), depth)?;
        info.compound_operator = Some(op);
        return Some(info);
    }

    parse_pipeline_or_simple(command, depth)
}

fn parse_pipeline_or_simple(command: &str, depth: usize) -> Option<CommandInfo> {
    let segments = split_top_level_pipeline(command);
    if segments.len() <= 1 {
        return parse_simple_command(command, depth);
    }

    let parsed_segments = segments
        .iter()
        .enumerate()
        .map(|(position, segment)| {
            parse_simple_command(segment.trim(), depth).map(|info| PipeSegment { info, position })
        })
        .collect::<Option<Vec<_>>>()?;

    let mut info = parsed_segments.first()?.info.clone();
    info.pipes = parsed_segments.clone();
    info.compound_operator = Some(CompoundOp::Pipe);
    // 管道风险来自所有段，首段只作为代表命令，特征字段必须从每个段汇总。
    info.redirects =
        parsed_segments.iter().flat_map(|segment| segment.info.redirects.clone()).collect();
    info.subcommands =
        parsed_segments.iter().flat_map(|segment| segment.info.subcommands.clone()).collect();
    info.has_command_substitution =
        parsed_segments.iter().any(|segment| segment.info.has_command_substitution);
    info.has_process_substitution =
        parsed_segments.iter().any(|segment| segment.info.has_process_substitution);
    info.has_glob = parsed_segments.iter().any(|segment| segment.info.has_glob);
    info.has_variable_expansion =
        parsed_segments.iter().any(|segment| segment.info.has_variable_expansion);
    Some(info)
}

fn parse_simple_command(command: &str, depth: usize) -> Option<CommandInfo> {
    let tokens = tokenize_segment(command, depth)?;
    if tokens.is_empty() {
        return None;
    }

    let mut name = None;
    let mut args = Vec::new();
    let mut redirects = Vec::new();
    let mut subcommands = Vec::new();
    let mut has_command_substitution = false;
    let mut has_process_substitution = false;
    let mut has_glob = false;
    let mut has_variable_expansion = false;
    let mut index = 0;

    while index < tokens.len() {
        if let Some((redirect, consumed)) = parse_redirect(&tokens, index) {
            redirects.push(redirect);
            index += consumed;
            continue;
        }

        let token = &tokens[index];
        has_command_substitution |= token.has_command_substitution;
        has_process_substitution |= token.has_process_substitution;
        has_glob |= token.has_glob;
        has_variable_expansion |= token.has_variable_expansion;
        subcommands.extend(token.subcommands.clone());

        if name.is_none() {
            name = Some(token.text.clone());
        } else {
            args.push(token.text.clone());
        }

        index += 1;
    }

    Some(CommandInfo {
        name: name?,
        args,
        redirects,
        pipes: Vec::new(),
        subcommands,
        has_command_substitution,
        has_process_substitution,
        has_glob,
        has_variable_expansion,
        compound_operator: None,
    })
}

fn tokenize_segment(segment: &str, depth: usize) -> Option<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut current = Token::default();
    let mut index = 0;

    while index < segment.len() {
        let ch = next_char(segment, index)?;

        if ch.is_whitespace() {
            push_token(&mut tokens, &mut current);
            index += ch.len_utf8();
            continue;
        }

        if let Some((compound_op, len)) = read_compound_operator(segment, index) {
            if matches!(compound_op, "&&" | "||" | "|" | ";") {
                // 复合操作符在上层已经切分，这里把它当作 token 边界以避免拼入参数。
                push_token(&mut tokens, &mut current);
                index += len;
                continue;
            }
        }

        if let Some((inner, end)) = read_process_substitution(segment, index) {
            current.text.push_str(&segment[index..end]);
            current.has_process_substitution = true;
            if inner.contains('*') || inner.contains('?') || inner.contains('[') {
                current.has_glob = true;
            }
            index = end;
            continue;
        }

        if let Some((inner, end)) = read_command_substitution(segment, index) {
            current.text.push_str(&segment[index..end]);
            current.has_command_substitution = true;
            if depth < MAX_SUBCOMMAND_DEPTH {
                if let Some(info) = parse_command_info(inner.trim(), depth + 1) {
                    current.subcommands.push(info);
                }
            }
            index = end;
            continue;
        }

        if ch == '`' {
            let (inner, end) = read_backticks(segment, index)?;
            current.text.push_str(&segment[index..end]);
            current.has_command_substitution = true;
            if depth < MAX_SUBCOMMAND_DEPTH {
                if let Some(info) = parse_command_info(inner.trim(), depth + 1) {
                    current.subcommands.push(info);
                }
            }
            index = end;
            continue;
        }

        if ch == '\'' {
            let (text, end) = read_single_quoted(segment, index)?;
            current.text.push_str(&text);
            index = end;
            continue;
        }

        if ch == '"' {
            let (token, end) = read_double_quoted(segment, index, depth)?;
            current.text.push_str(&token.text);
            current.has_command_substitution |= token.has_command_substitution;
            current.has_process_substitution |= token.has_process_substitution;
            current.has_glob |= token.has_glob;
            current.has_variable_expansion |= token.has_variable_expansion;
            current.subcommands.extend(token.subcommands);
            index = end;
            continue;
        }

        if ch == '\\' {
            let next = next_char(segment, index + ch.len_utf8())?;
            current.text.push(next);
            index += ch.len_utf8() + next.len_utf8();
            continue;
        }

        if ch == '$' {
            if let Some((expansion, end)) = read_variable_expansion(segment, index) {
                current.text.push_str(&expansion);
                current.has_variable_expansion = true;
                index = end;
                continue;
            }
        }

        if ch == '<' || ch == '>' {
            if current.text.is_empty() || current.text.chars().all(|value| value.is_ascii_digit()) {
                // 只有空 token 或纯数字前缀才归入重定向，避免把普通参数中的尖括号误判。
                let (redirect, end) = read_redirect_token(segment, index, &current.text);
                current.text = redirect;
                push_token(&mut tokens, &mut current);
                index = end;
                continue;
            }

            push_token(&mut tokens, &mut current);
            let (redirect, end) = read_redirect_token(segment, index, "");
            current.text = redirect;
            push_token(&mut tokens, &mut current);
            index = end;
            continue;
        }

        if matches!(ch, '*' | '?' | '[') {
            current.has_glob = true;
        }

        current.text.push(ch);
        index += ch.len_utf8();
    }

    push_token(&mut tokens, &mut current);
    Some(tokens)
}

fn parse_redirect(tokens: &[Token], index: usize) -> Option<(Redirect, usize)> {
    let token = tokens.get(index)?.text.as_str();
    if token.starts_with("<(") || token.starts_with(">(") {
        // 进程替换形如 <(...)，语义不是文件重定向，必须留给特征标记处理。
        return None;
    }

    if let Some(target) = token.strip_prefix("2>&") {
        return Some((
            Redirect {
                kind: RedirectKind::Stderr,
                target: target.to_string(),
                is_fd_duplicate: true,
            },
            1,
        ));
    }

    if let Some(target) = token.strip_prefix("1>&") {
        return Some((
            Redirect {
                kind: RedirectKind::Stdout,
                target: target.to_string(),
                is_fd_duplicate: true,
            },
            1,
        ));
    }

    if let Some(target) = token.strip_prefix("<&") {
        return Some((
            Redirect {
                kind: RedirectKind::Stdin,
                target: target.to_string(),
                is_fd_duplicate: true,
            },
            1,
        ));
    }

    if let Some(target) = token.strip_prefix("2>>") {
        return Some((
            build_redirect(RedirectKind::StderrAppend, target, tokens.get(index + 1), false)?,
            if target.is_empty() { 2 } else { 1 },
        ));
    }

    if let Some(target) = token.strip_prefix("2>") {
        return Some((
            build_redirect(RedirectKind::Stderr, target, tokens.get(index + 1), false)?,
            if target.is_empty() { 2 } else { 1 },
        ));
    }

    if let Some(target) = token.strip_prefix("&>>") {
        return Some((
            build_redirect(RedirectKind::StderrAppend, target, tokens.get(index + 1), false)?,
            if target.is_empty() { 2 } else { 1 },
        ));
    }

    if let Some(target) = token.strip_prefix("&>") {
        return Some((
            build_redirect(RedirectKind::StdoutAndStderr, target, tokens.get(index + 1), false)?,
            if target.is_empty() { 2 } else { 1 },
        ));
    }

    if let Some(target) = token.strip_prefix(">>") {
        return Some((
            build_redirect(RedirectKind::Append, target, tokens.get(index + 1), false)?,
            if target.is_empty() { 2 } else { 1 },
        ));
    }

    if let Some(target) = token.strip_prefix("<<<") {
        return Some((
            build_redirect(RedirectKind::Heredoc, target, tokens.get(index + 1), false)?,
            if target.is_empty() { 2 } else { 1 },
        ));
    }

    if let Some(target) = token.strip_prefix("<<") {
        return Some((
            build_redirect(RedirectKind::Heredoc, target, tokens.get(index + 1), false)?,
            if target.is_empty() { 2 } else { 1 },
        ));
    }

    if let Some(target) = token.strip_prefix('>') {
        return Some((
            build_redirect(RedirectKind::Stdout, target, tokens.get(index + 1), false)?,
            if target.is_empty() { 2 } else { 1 },
        ));
    }

    if let Some(target) = token.strip_prefix('<') {
        return Some((
            build_redirect(RedirectKind::Stdin, target, tokens.get(index + 1), false)?,
            if target.is_empty() { 2 } else { 1 },
        ));
    }

    None
}

fn build_redirect(
    kind: RedirectKind,
    inline_target: &str,
    next: Option<&Token>,
    is_fd_duplicate: bool,
) -> Option<Redirect> {
    let target =
        if inline_target.is_empty() { next?.text.clone() } else { inline_target.to_string() };

    Some(Redirect { kind, target, is_fd_duplicate })
}

fn split_top_level_compound(command: &str) -> Option<(&str, CompoundOp, &str)> {
    let mut index = 0;
    while index < command.len() {
        if let Some((_, end)) = read_process_substitution(command, index) {
            // 替换表达式内部可能包含 &&、|| 或 ;，跳过整段才能只识别顶层操作符。
            index = end;
            continue;
        }
        if let Some((_, end)) = read_command_substitution(command, index) {
            index = end;
            continue;
        }

        let ch = next_char(command, index)?;
        match ch {
            '\'' => {
                let (_, end) = read_single_quoted(command, index)?;
                index = end;
            }
            '"' => {
                let (_, end) = read_double_quoted(command, index, 0)?;
                index = end;
            }
            '`' => {
                let (_, end) = read_backticks(command, index)?;
                index = end;
            }
            '&' if command[index..].starts_with("&&") => {
                return Some((&command[..index], CompoundOp::And, &command[index + 2..]));
            }
            '|' if command[index..].starts_with("||") => {
                return Some((&command[..index], CompoundOp::Or, &command[index + 2..]));
            }
            ';' => return Some((&command[..index], CompoundOp::Sequence, &command[index + 1..])),
            _ => index += ch.len_utf8(),
        }
    }
    None
}

fn split_top_level_pipeline(command: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut index = 0;

    while index < command.len() {
        if let Some((_, end)) = read_process_substitution(command, index) {
            index = end;
            continue;
        }
        if let Some((_, end)) = read_command_substitution(command, index) {
            index = end;
            continue;
        }

        let Some(ch) = next_char(command, index) else {
            break;
        };

        match ch {
            '\'' => {
                if let Some((_, end)) = read_single_quoted(command, index) {
                    index = end;
                } else {
                    break;
                }
            }
            '"' => {
                if let Some((_, end)) = read_double_quoted(command, index, 0) {
                    index = end;
                } else {
                    break;
                }
            }
            '`' => {
                if let Some((_, end)) = read_backticks(command, index) {
                    index = end;
                } else {
                    break;
                }
            }
            '|' if !command[index..].starts_with("||") => {
                segments.push(command[start..index].trim());
                start = index + 1;
                index += 1;
            }
            _ => index += ch.len_utf8(),
        }
    }

    segments.push(command[start..].trim());
    segments
}

fn subshell_inner(command: &str) -> Option<&str> {
    if !command.starts_with('(') || !command.ends_with(')') {
        return None;
    }

    let mut depth = 0usize;
    for (index, ch) in command.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && index + 1 != command.len() {
                    return None;
                }
            }
            _ => {}
        }
    }

    if depth == 0 { Some(&command[1..command.len() - 1]) } else { None }
}

fn push_token(tokens: &mut Vec<Token>, current: &mut Token) {
    if !current.text.is_empty() {
        tokens.push(std::mem::take(current));
    }
}

fn next_char(input: &str, index: usize) -> Option<char> {
    input.get(index..)?.chars().next()
}

fn read_single_quoted(input: &str, start: usize) -> Option<(String, usize)> {
    let mut index = start + 1;
    while index < input.len() {
        let ch = next_char(input, index)?;
        if ch == '\'' {
            return Some((input[start + 1..index].to_string(), index + 1));
        }
        index += ch.len_utf8();
    }
    None
}

fn read_double_quoted(input: &str, start: usize, depth: usize) -> Option<(Token, usize)> {
    let mut token = Token::default();
    let mut index = start + 1;
    while index < input.len() {
        if let Some((inner, end)) = read_command_substitution(input, index) {
            token.text.push_str(&input[index..end]);
            token.has_command_substitution = true;
            if depth < MAX_SUBCOMMAND_DEPTH {
                if let Some(info) = parse_command_info(inner.trim(), depth + 1) {
                    token.subcommands.push(info);
                }
            }
            index = end;
            continue;
        }
        if let Some((_, end)) = read_process_substitution(input, index) {
            token.text.push_str(&input[index..end]);
            token.has_process_substitution = true;
            index = end;
            continue;
        }

        let ch = next_char(input, index)?;
        match ch {
            '"' => return Some((token, index + 1)),
            '\\' => {
                let next = next_char(input, index + 1)?;
                token.text.push(next);
                index += 1 + next.len_utf8();
            }
            '$' => {
                if let Some((expansion, end)) = read_variable_expansion(input, index) {
                    token.text.push_str(&expansion);
                    token.has_variable_expansion = true;
                    index = end;
                } else {
                    token.text.push(ch);
                    index += 1;
                }
            }
            _ => {
                token.text.push(ch);
                index += ch.len_utf8();
            }
        }
    }
    None
}

fn read_backticks(input: &str, start: usize) -> Option<(&str, usize)> {
    let mut index = start + 1;
    while index < input.len() {
        let ch = next_char(input, index)?;
        if ch == '\\' {
            let next = next_char(input, index + 1)?;
            index += 1 + next.len_utf8();
            continue;
        }
        if ch == '`' {
            return Some((&input[start + 1..index], index + 1));
        }
        index += ch.len_utf8();
    }
    None
}

fn read_command_substitution(input: &str, start: usize) -> Option<(&str, usize)> {
    if !input[start..].starts_with("$(") {
        return None;
    }
    read_parenthesized(input, start + 2).map(|(inner, end)| (inner, end))
}

fn read_process_substitution(input: &str, start: usize) -> Option<(&str, usize)> {
    if input[start..].starts_with("<(") || input[start..].starts_with(">(") {
        return read_parenthesized(input, start + 2).map(|(inner, end)| (inner, end));
    }
    None
}

fn read_parenthesized(input: &str, mut index: usize) -> Option<(&str, usize)> {
    let content_start = index;
    let mut depth = 1usize;
    while index < input.len() {
        if let Some((_, end)) = read_command_substitution(input, index) {
            index = end;
            continue;
        }
        if let Some((_, end)) = read_process_substitution(input, index) {
            index = end;
            continue;
        }

        let ch = next_char(input, index)?;
        match ch {
            '\'' => {
                let (_, end) = read_single_quoted(input, index)?;
                index = end;
            }
            '"' => {
                let (_, end) = read_double_quoted(input, index, 0)?;
                index = end;
            }
            '`' => {
                let (_, end) = read_backticks(input, index)?;
                index = end;
            }
            '(' => {
                depth += 1;
                index += 1;
            }
            ')' => {
                depth -= 1;
                index += 1;
                if depth == 0 {
                    return Some((&input[content_start..index - 1], index));
                }
            }
            '\\' => {
                let next = next_char(input, index + 1)?;
                index += 1 + next.len_utf8();
            }
            _ => index += ch.len_utf8(),
        }
    }
    None
}

fn read_variable_expansion(input: &str, start: usize) -> Option<(String, usize)> {
    if !input[start..].starts_with('$') {
        return None;
    }

    let next = next_char(input, start + 1)?;
    if next == '{' {
        let end = input[start + 2..].find('}')?;
        let end_index = start + 2 + end;
        return Some((input[start..=end_index].to_string(), end_index + 1));
    }

    if matches!(next, '?' | '$' | '!' | '#' | '*' | '@') || next.is_ascii_digit() {
        return Some((
            input[start..start + 1 + next.len_utf8()].to_string(),
            start + 1 + next.len_utf8(),
        ));
    }

    if next == '_' || next.is_ascii_alphabetic() {
        let mut index = start + 1 + next.len_utf8();
        while index < input.len() {
            let ch = next_char(input, index)?;
            if ch == '_' || ch.is_ascii_alphanumeric() {
                index += ch.len_utf8();
            } else {
                break;
            }
        }
        return Some((input[start..index].to_string(), index));
    }

    None
}

fn read_compound_operator(input: &str, start: usize) -> Option<(&'static str, usize)> {
    if input[start..].starts_with("&&") {
        Some(("&&", 2))
    } else if input[start..].starts_with("||") {
        Some(("||", 2))
    } else if input[start..].starts_with('|') {
        Some(("|", 1))
    } else if input[start..].starts_with(';') {
        Some((";", 1))
    } else {
        None
    }
}

fn read_redirect_token(input: &str, start: usize, prefix: &str) -> (String, usize) {
    let mut token = String::from(prefix);
    let mut index = start;
    while index < input.len() {
        let Some(ch) = next_char(input, index) else {
            break;
        };
        if ch.is_whitespace() || matches!(ch, '\'' | '"' | '`' | ';' | '|') {
            break;
        }
        if (ch == '<' || ch == '>')
            && !token.is_empty()
            && !token.chars().all(|value| value.is_ascii_digit())
        {
            break;
        }
        token.push(ch);
        index += ch.len_utf8();
        if token.contains(">&") || token.contains("<&") {
            while index < input.len() {
                let Some(next) = next_char(input, index) else {
                    break;
                };
                if next.is_whitespace() {
                    break;
                }
                token.push(next);
                index += next.len_utf8();
            }
            break;
        }
        if matches!(token.as_str(), ">" | ">>" | "<" | "<<" | "<<<" | "2>" | "2>>" | "&>" | "&>>") {
            if matches!(token.as_str(), "2>" | ">") && input[index..].starts_with('&') {
                continue;
            }
            break;
        }
    }
    (token, index)
}
#[cfg(test)]
#[path = "scan_tests.rs"]
mod scan_tests;
