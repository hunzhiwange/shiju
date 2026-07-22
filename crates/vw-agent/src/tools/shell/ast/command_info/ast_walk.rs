//! 基于 Bash AST 的命令信息提取。
//!
//! 本模块在启用 `shell-ast` 特性时使用 tree-sitter 节点结构直接识别命令、管道、
//! 重定向和替换表达式。相比字符串扫描，AST 路径能更可靠地区分语法节点边界。

use super::super::nodes::{CommandInfo, CompoundOp, PipeSegment, Redirect, RedirectKind};
use super::super::parser::{BashAst, BashNode};
use super::MAX_SUBCOMMAND_DEPTH;

pub(super) fn from_ast(ast: &BashAst) -> Option<CommandInfo> {
    let Some(root) = ast.root_node() else {
        return CommandInfo::from_command(ast.source());
    };
    let mut cursor = root.walk();
    root.named_children(&mut cursor)
        .find_map(|child| build_command_info_from_node(ast, child, 0))
        .or_else(|| CommandInfo::from_command(ast.source()))
}

fn build_command_info_from_node(
    ast: &BashAst,
    node: BashNode<'_>,
    depth: usize,
) -> Option<CommandInfo> {
    if depth > MAX_SUBCOMMAND_DEPTH {
        // 命令替换可以递归嵌套；深度上限让分析保持有界，避免恶意输入拖垮工具调用。
        return None;
    }

    match node.kind() {
        "program" | "compound_statement" | "do_group" | "list" => first_named_child(node)
            .and_then(|child| build_command_info_from_node(ast, child, depth)),
        "pipeline" => build_pipeline(ast, node, depth),
        "redirected_statement" => build_redirected_statement(ast, node, depth),
        "subshell" => build_subshell(ast, node, depth),
        "command" => build_simple_command(ast, node, depth),
        "declaration_command" | "unset_command" | "test_command" => {
            build_keyword_command(ast, node, depth)
        }
        "negated_command" => first_named_child(node)
            .and_then(|child| build_command_info_from_node(ast, child, depth)),
        _ => None,
    }
}

fn build_pipeline(ast: &BashAst, node: BashNode<'_>, depth: usize) -> Option<CommandInfo> {
    let segments = named_children(node)
        .into_iter()
        .enumerate()
        .filter_map(|(position, child)| {
            build_command_info_from_node(ast, child, depth + 1)
                .map(|info| PipeSegment { info, position })
        })
        .collect::<Vec<_>>();

    let mut info = segments.first()?.info.clone();
    info.pipes = segments.clone();
    info.compound_operator = Some(CompoundOp::Pipe);
    // 管道整体以首段命令为主命令，同时汇总各段的安全相关特征，供上层策略一次判断。
    info.redirects = segments.iter().flat_map(|segment| segment.info.redirects.clone()).collect();
    info.subcommands =
        segments.iter().flat_map(|segment| segment.info.subcommands.clone()).collect();
    info.has_command_substitution =
        segments.iter().any(|segment| segment.info.has_command_substitution);
    info.has_process_substitution =
        segments.iter().any(|segment| segment.info.has_process_substitution);
    info.has_glob = segments.iter().any(|segment| segment.info.has_glob);
    info.has_variable_expansion =
        segments.iter().any(|segment| segment.info.has_variable_expansion);
    Some(info)
}

fn build_redirected_statement(
    ast: &BashAst,
    node: BashNode<'_>,
    depth: usize,
) -> Option<CommandInfo> {
    let body = node.child_by_field_name("body")?;
    let mut info = build_command_info_from_node(ast, body, depth + 1)?;
    let redirects = collect_redirects(ast, node);
    if !redirects.is_empty() {
        info.redirects.extend(redirects);
    }
    Some(info)
}

fn build_subshell(ast: &BashAst, node: BashNode<'_>, depth: usize) -> Option<CommandInfo> {
    let child = first_named_child(node)?;
    let mut info = build_command_info_from_node(ast, child, depth + 1)?;
    info.compound_operator = Some(CompoundOp::Subshell);
    Some(info)
}

fn build_simple_command(ast: &BashAst, node: BashNode<'_>, depth: usize) -> Option<CommandInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = extract_text(ast, name_node);
    if name.trim().is_empty() {
        return None;
    }

    let args = field_children(node, "argument")
        .into_iter()
        .map(|child| flatten_word(ast, child))
        .filter(|value: &String| !value.is_empty())
        .collect::<Vec<_>>();

    let redirects = field_children(node, "redirect")
        .into_iter()
        .filter_map(|child| parse_redirect_node(ast, child))
        .collect::<Vec<_>>();

    let (
        subcommands,
        has_command_substitution,
        has_process_substitution,
        has_glob,
        has_variable_expansion,
    ) = analyze_node_features(ast, node, depth + 1);

    Some(CommandInfo {
        name: flatten_word(ast, name_node),
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

fn build_keyword_command(ast: &BashAst, node: BashNode<'_>, depth: usize) -> Option<CommandInfo> {
    let name = extract_text(ast, node).split_whitespace().next()?.to_string();
    let mut args = named_children(node)
        .into_iter()
        .map(|child| flatten_word(ast, child))
        .filter(|value: &String| !value.is_empty())
        .collect::<Vec<_>>();
    if args.first().is_some_and(|value| value == &name) {
        args.remove(0);
    }

    let (
        subcommands,
        has_command_substitution,
        has_process_substitution,
        has_glob,
        has_variable_expansion,
    ) = analyze_node_features(ast, node, depth + 1);

    Some(CommandInfo {
        name,
        args,
        redirects: Vec::new(),
        pipes: Vec::new(),
        subcommands,
        has_command_substitution,
        has_process_substitution,
        has_glob,
        has_variable_expansion,
        compound_operator: None,
    })
}

fn analyze_node_features(
    ast: &BashAst,
    node: BashNode<'_>,
    depth: usize,
) -> (Vec<CommandInfo>, bool, bool, bool, bool) {
    let mut subcommands = Vec::new();
    let mut has_command_substitution = false;
    let mut has_process_substitution = false;
    let mut has_glob = false;
    let mut has_variable_expansion = false;

    walk_descendants(node, &mut |child| match child.kind() {
        "command_substitution" => {
            has_command_substitution = true;
            if depth <= MAX_SUBCOMMAND_DEPTH {
                // 子命令会影响实际执行面，保留它们便于安全策略看到嵌套调用。
                if let Some(info) = first_named_child(child)
                    .and_then(|stmt| build_command_info_from_node(ast, stmt, depth + 1))
                {
                    subcommands.push(info);
                }
            }
        }
        "process_substitution" => {
            has_process_substitution = true;
            if depth <= MAX_SUBCOMMAND_DEPTH {
                if let Some(info) = first_named_child(child)
                    .and_then(|stmt| build_command_info_from_node(ast, stmt, depth + 1))
                {
                    subcommands.push(info);
                }
            }
        }
        "expansion" | "simple_expansion" => {
            has_variable_expansion = true;
        }
        "word" | "concatenation" | "string" | "raw_string" | "string_content" => {
            if contains_glob_pattern(&extract_text(ast, child)) {
                has_glob = true;
            }
        }
        _ => {}
    });

    (
        subcommands,
        has_command_substitution,
        has_process_substitution,
        has_glob,
        has_variable_expansion,
    )
}

fn collect_redirects(ast: &BashAst, node: BashNode<'_>) -> Vec<Redirect> {
    let mut seen = std::collections::HashSet::new();
    let mut redirects = Vec::new();

    for child in field_children(node, "redirect") {
        if seen.insert((child.start_byte(), child.end_byte())) {
            if let Some(redirect) = parse_redirect_node(ast, child) {
                redirects.push(redirect);
            }
        }
    }

    walk_descendants(node, &mut |child| {
        if matches!(child.kind(), "file_redirect" | "herestring_redirect" | "heredoc_redirect")
            && seen.insert((child.start_byte(), child.end_byte()))
        {
            if let Some(redirect) = parse_redirect_node(ast, child) {
                redirects.push(redirect);
            }
        }
    });

    redirects
}

fn parse_redirect_node(ast: &BashAst, node: BashNode<'_>) -> Option<Redirect> {
    match node.kind() {
        "file_redirect" => parse_file_redirect(ast, node),
        "herestring_redirect" => Some(Redirect {
            kind: RedirectKind::Heredoc,
            target: named_children(node)
                .into_iter()
                .map(|child| flatten_word(ast, child))
                .collect::<String>(),
            is_fd_duplicate: false,
        }),
        "heredoc_redirect" => {
            let mut target = field_children(node, "argument")
                .into_iter()
                .map(|child| flatten_word(ast, child))
                .collect::<String>();
            if target.is_empty() {
                target = extract_text(ast, node)
                    .split_once("<<")
                    .map(|(_, rhs)| rhs.split_whitespace().next().unwrap_or_default())
                    .unwrap_or_default()
                    .to_string();
            }
            Some(Redirect { kind: RedirectKind::Heredoc, target, is_fd_duplicate: false })
        }
        _ => None,
    }
}

fn parse_file_redirect(ast: &BashAst, node: BashNode<'_>) -> Option<Redirect> {
    let text = extract_text(ast, node);
    let descriptor = node
        .child_by_field_name("descriptor")
        .map(|child| extract_text(ast, child))
        .unwrap_or_default();
    let destination = field_children(node, "destination")
        .into_iter()
        .map(|child| flatten_word(ast, child))
        .collect::<String>();

    let kind = if text.contains("&>>") || descriptor == "2" && text.contains(">>") {
        RedirectKind::StderrAppend
    } else if text.contains("&>") {
        RedirectKind::StdoutAndStderr
    } else if descriptor == "2" && text.contains(">>") {
        RedirectKind::StderrAppend
    } else if descriptor == "2" && text.contains('>') {
        RedirectKind::Stderr
    } else if text.contains(">>") {
        RedirectKind::Append
    } else if text.contains("<<") {
        RedirectKind::Heredoc
    } else if text.contains('<') {
        RedirectKind::Stdin
    } else {
        RedirectKind::Stdout
    };

    let is_fd_duplicate = text.contains(">&") || text.contains("<&");
    let target = if destination.is_empty() {
        text.split_once('>')
            .map(|(_, rhs)| rhs)
            .or_else(|| text.split_once('<').map(|(_, rhs)| rhs))
            .unwrap_or_default()
            .trim()
            .to_string()
    } else {
        destination
    };

    Some(Redirect { kind, target, is_fd_duplicate })
}

fn flatten_word(ast: &BashAst, node: BashNode<'_>) -> String {
    match node.kind() {
        "string" => named_children(node)
            .into_iter()
            .map(|child| flatten_word(ast, child))
            .collect::<String>(),
        "command_name" => {
            let children = named_children(node);
            if children.is_empty() {
                extract_text(ast, node)
            } else {
                children.into_iter().map(|child| flatten_word(ast, child)).collect::<String>()
            }
        }
        "concatenation" => named_children(node)
            .into_iter()
            .map(|child| flatten_word(ast, child))
            .collect::<String>(),
        "raw_string"
        | "word"
        | "string_content"
        | "special_variable_name"
        | "variable_name"
        | "file_descriptor" => unquote_shell_leaf(&extract_text(ast, node)),
        "simple_expansion" | "expansion" | "arithmetic_expansion" | "parenthesized_expression" => {
            extract_text(ast, node)
        }
        "command_substitution" | "process_substitution" => extract_text(ast, node),
        _ => {
            let children = named_children(node);
            if children.is_empty() {
                extract_text(ast, node)
            } else {
                children.into_iter().map(|child| flatten_word(ast, child)).collect::<String>()
            }
        }
    }
}

fn extract_text(ast: &BashAst, node: BashNode<'_>) -> String {
    let range = node.byte_range();
    ast.source()
        .as_bytes()
        .get(range)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .unwrap_or_default()
        .to_string()
}

fn unquote_shell_leaf(value: &str) -> String {
    if value.len() >= 2
        && ((value.starts_with('\'') && value.ends_with('\''))
            || (value.starts_with('"') && value.ends_with('"')))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

fn field_children<'tree>(node: BashNode<'tree>, field_name: &str) -> Vec<BashNode<'tree>> {
    let mut cursor = node.walk();
    node.children_by_field_name(field_name, &mut cursor).collect()
}

fn named_children<'tree>(node: BashNode<'tree>) -> Vec<BashNode<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn first_named_child<'tree>(node: BashNode<'tree>) -> Option<BashNode<'tree>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).next()
}

fn walk_descendants<'tree>(node: BashNode<'tree>, visit: &mut impl FnMut(BashNode<'tree>)) {
    for child in named_children(node) {
        visit(child);
        walk_descendants(child, visit);
    }
}

fn contains_glob_pattern(value: &str) -> bool {
    value.chars().any(|ch| matches!(ch, '*' | '?' | '['))
}
#[cfg(test)]
#[path = "ast_walk_tests.rs"]
mod ast_walk_tests;
