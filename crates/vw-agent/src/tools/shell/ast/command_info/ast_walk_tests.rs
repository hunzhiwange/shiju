//! 验证从 Bash 语法树提取命令、重定向、子命令及展开特征的行为。

use super::*;

#[test]
fn from_ast_unwraps_negated_command() {
    let (ast, _) = BashAst::parse("! grep needle input.txt");
    let info = from_ast(&ast).expect("negated command should expose its inner command");

    assert_eq!(info.name, "grep");
    assert_eq!(info.args, vec!["needle", "input.txt"]);
}

#[test]
fn from_ast_marks_subshell_and_detects_glob() {
    let (ast, _) = BashAst::parse("(printf '%s' *.rs)");
    let info = from_ast(&ast).expect("subshell should contain a command");

    assert_eq!(info.name, "printf");
    assert_eq!(info.compound_operator, Some(CompoundOp::Subshell));
    assert!(info.has_glob);
}

#[test]
fn from_ast_detects_glob_and_process_substitution_subcommands() {
    let (ast, _) = BashAst::parse("diff <(sort left*) >(cat \"$RIGHT\") literal?.txt");
    let info = from_ast(&ast).expect("process substitution command should parse");

    assert_eq!(info.name, "diff");
    assert!(info.has_process_substitution);
    assert!(info.has_variable_expansion);
    assert!(info.has_glob);
    assert_eq!(
        info.subcommands.iter().map(|command| command.name.as_str()).collect::<Vec<_>>(),
        vec!["sort", "cat"]
    );
}

#[test]
fn from_ast_aggregates_pipeline_segments_redirects_and_features() {
    let (ast, _) = BashAst::parse("printf \"$VALUE\" | grep *.rs");
    let info = from_ast(&ast).expect("pipeline should parse");

    assert_eq!(info.name, "printf");
    assert_eq!(info.compound_operator, Some(CompoundOp::Pipe));
    assert_eq!(info.pipes.len(), 2);
    assert_eq!(info.pipes[1].info.name, "grep");
    assert!(info.has_variable_expansion);
    assert!(info.has_glob);
    assert!(info.redirects.is_empty());
}

#[test]
fn from_ast_collects_command_substitution_subcommands() {
    let (ast, _) = BashAst::parse("echo $(printf inner)");
    let info = from_ast(&ast).expect("command substitution should parse");

    assert!(info.has_command_substitution);
    assert_eq!(info.subcommands.len(), 1);
    assert_eq!(info.subcommands[0].name, "printf");
    assert_eq!(info.args, vec!["$(printf inner)"]);
}

#[test]
fn from_ast_reads_do_group_redirects_and_heredocs() {
    let (ast, _) = BashAst::parse("{ echo hi; } > out.txt");
    let info = from_ast(&ast).expect("redirected do_group should parse");

    assert_eq!(info.name, "echo");
    assert_eq!(info.redirects.len(), 1);
    assert_eq!(info.redirects[0].kind, RedirectKind::Stdout);
    assert_eq!(info.redirects[0].target, "out.txt");

    let (ast, _) = BashAst::parse("cat <<EOF\nhello\nEOF");
    let info = from_ast(&ast).expect("heredoc command should parse");
    let redirect = info.redirects.first().expect("heredoc should be collected");

    assert_eq!(redirect.kind, RedirectKind::Heredoc);
    assert_eq!(redirect.target, "EOF");
}

#[test]
fn from_ast_falls_back_when_root_node_is_not_supported() {
    let (ast, _) = BashAst::parse("if true; then echo hi; fi");
    let info = from_ast(&ast).expect("unsupported AST node should fall back to scanner");

    assert_eq!(info.name, "if");
    assert_eq!(info.args.first().map(String::as_str), Some("true"));
}

#[test]
fn from_ast_builds_keyword_commands_with_arguments_and_expansions() {
    let (ast, _) = BashAst::parse("unset \"$NAME\"");
    let info = from_ast(&ast).expect("keyword command should parse");

    assert_eq!(info.name, "unset");
    assert_eq!(info.args, vec!["$NAME"]);
    assert!(info.has_variable_expansion);
}

#[test]
fn from_ast_classifies_file_redirect_kinds() {
    for (command, kind, target, is_fd_duplicate) in [
        ("printf hi > out.txt", RedirectKind::Stdout, "out.txt", false),
        ("printf hi >> out.txt", RedirectKind::Append, "out.txt", false),
        ("printf hi < in.txt", RedirectKind::Stdin, "in.txt", false),
        ("printf hi 2> err.txt", RedirectKind::Stderr, "err.txt", false),
        ("printf hi 2>> err.txt", RedirectKind::StderrAppend, "err.txt", false),
        ("printf hi &> both.txt", RedirectKind::StdoutAndStderr, "both.txt", false),
        ("printf hi &>> both.txt", RedirectKind::StderrAppend, "both.txt", false),
        ("printf hi 1>&2", RedirectKind::Stdout, "2", true),
    ] {
        let (ast, _) = BashAst::parse(command);
        let info = from_ast(&ast).expect("redirect command should parse");
        let redirect = info.redirects.first().expect("redirect should be collected");

        assert_eq!(redirect.kind, kind, "command: {command}");
        assert_eq!(redirect.target, target, "command: {command}");
        assert_eq!(redirect.is_fd_duplicate, is_fd_duplicate, "command: {command}");
    }
}

#[test]
fn from_ast_collects_herestring_redirects() {
    let (ast, _) = BashAst::parse("cat <<< \"$VALUE\"");
    let info = from_ast(&ast).expect("herestring command should parse");
    let redirect = info.redirects.first().expect("herestring should be collected");

    assert_eq!(redirect.kind, RedirectKind::Heredoc);
    assert_eq!(redirect.target, "$VALUE");
    assert!(!redirect.is_fd_duplicate);
}

#[test]
fn shell_leaf_helpers_preserve_unquoted_and_decode_quoted_values() {
    assert_eq!(unquote_shell_leaf("'hello world'"), "hello world");
    assert_eq!(unquote_shell_leaf("\"hello world\""), "hello world");
    assert_eq!(unquote_shell_leaf("plain"), "plain");
    assert_eq!(unquote_shell_leaf("'"), "'");
}

#[test]
fn glob_detection_recognizes_each_supported_metacharacter() {
    assert!(contains_glob_pattern("*.rs"));
    assert!(contains_glob_pattern("file?.txt"));
    assert!(contains_glob_pattern("[ab].log"));
    assert!(!contains_glob_pattern("literal.txt"));
}
