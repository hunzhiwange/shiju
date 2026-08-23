//! 验证命令文本扫描器对管道、引号、展开、重定向与嵌套边界的解析。

use super::*;

fn token(text: &str) -> Token {
    Token { text: text.to_string(), ..Token::default() }
}

#[test]
fn parse_command_info_handles_pipeline_and_redirect() {
    let info = parse_command_info("cat file.txt | grep needle > out.txt", 0).unwrap();
    assert_eq!(info.name, "cat");
    assert_eq!(info.pipes.len(), 2);
    assert_eq!(info.redirects.len(), 1);
}

#[test]
fn tokenize_segment_preserves_quoted_argument() {
    let info = parse_command_info("echo 'hello world'", 0).unwrap();
    assert_eq!(info.name, "echo");
    assert_eq!(info.args, vec!["hello world".to_string()]);
}

#[test]
fn parse_command_info_rejects_empty_excessive_depth_and_incomplete_pipeline() {
    assert!(parse_command_info("", 0).is_none());
    assert!(parse_command_info("   ", 0).is_none());
    assert!(parse_command_info("echo", MAX_SUBCOMMAND_DEPTH + 1).is_none());
    assert!(parse_command_info("echo ok |", 0).is_none());
    assert!(parse_command_info("| cat", 0).is_none());
    assert!(parse_command_info("> output.txt", 0).is_none());
}

#[test]
fn parse_command_info_marks_each_top_level_compound_operator() {
    let cases = [
        ("first && second", CompoundOp::And),
        ("first || second", CompoundOp::Or),
        ("first; second", CompoundOp::Sequence),
    ];

    for (command, expected_operator) in cases {
        let info = parse_command_info(command, 0).unwrap();
        assert_eq!(info.name, "first", "command: {command}");
        assert_eq!(info.compound_operator, Some(expected_operator), "command: {command}");
    }
}

#[test]
fn parse_command_info_parses_balanced_subshell() {
    let info = parse_command_info("(echo nested)", 0).unwrap();

    assert_eq!(info.name, "echo");
    assert_eq!(info.args, vec!["nested".to_string()]);
    assert_eq!(info.compound_operator, Some(CompoundOp::Subshell));
}

#[test]
fn pipeline_aggregates_segments_redirects_and_expansion_flags() {
    let info =
        parse_command_info("printf \"$VALUE\" | grep *.rs 2> errors.log | cat < input.txt", 0)
            .unwrap();

    assert_eq!(info.name, "printf");
    assert_eq!(info.args, vec!["$VALUE".to_string()]);
    assert_eq!(info.compound_operator, Some(CompoundOp::Pipe));
    assert_eq!(
        info.pipes.iter().map(|segment| segment.position).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(info.pipes[1].info.name, "grep");
    assert_eq!(info.pipes[1].info.args, vec!["*.rs".to_string()]);
    assert_eq!(info.pipes[2].info.name, "cat");
    assert_eq!(info.redirects.len(), 2);
    assert_eq!(info.redirects[0].kind, RedirectKind::Stderr);
    assert_eq!(info.redirects[0].target, "errors.log");
    assert_eq!(info.redirects[1].kind, RedirectKind::Stdin);
    assert_eq!(info.redirects[1].target, "input.txt");
    assert!(info.has_glob);
    assert!(info.has_variable_expansion);
    assert!(!info.has_command_substitution);
    assert!(!info.has_process_substitution);
}

#[test]
fn pipeline_aggregates_subcommands_from_every_segment() {
    let info = parse_command_info("echo $(printf one) | cat `date`", 0).unwrap();

    assert!(info.has_command_substitution);
    assert_eq!(
        info.subcommands.iter().map(|command| command.name.as_str()).collect::<Vec<_>>(),
        vec!["printf", "date"]
    );
}

#[test]
fn depth_limit_keeps_substitution_flag_without_recursing() {
    let info = parse_command_info("echo $(date)", MAX_SUBCOMMAND_DEPTH).unwrap();

    assert!(info.has_command_substitution);
    assert!(info.subcommands.is_empty());
}

#[test]
fn parse_command_info_tracks_process_substitution_without_redirecting() {
    let info = parse_command_info("diff <(sort left*) >(cat)", 0).unwrap();

    assert_eq!(info.name, "diff");
    assert_eq!(info.args, vec!["<(sort left*)".to_string(), ">(cat)".to_string()]);
    assert!(info.redirects.is_empty());
    assert!(info.subcommands.is_empty());
    assert!(info.has_process_substitution);
    assert!(info.has_glob);
}

#[test]
fn quoted_expansions_preserve_text_without_marking_quoted_globs() {
    let info = parse_command_info(r#"echo '$HOME *.rs' "$USER *.txt" plain\ value"#, 0).unwrap();

    assert_eq!(
        info.args,
        vec!["$HOME *.rs".to_string(), "$USER *.txt".to_string(), "plain value".to_string(),]
    );
    assert!(info.has_variable_expansion);
    assert!(!info.has_glob);
}

#[test]
fn double_quotes_track_nested_substitutions_and_escaped_characters() {
    let info = parse_command_info(r#"echo "prefix $(printf inner) <(cat) a\"b""#, 0).unwrap();

    assert_eq!(info.args, vec![r#"prefix $(printf inner) <(cat) a"b"#.to_string()]);
    assert!(info.has_command_substitution);
    assert!(info.has_process_substitution);
    assert_eq!(info.subcommands.len(), 1);
    assert_eq!(info.subcommands[0].name, "printf");
    assert!(info.redirects.is_empty());
}

#[test]
fn supported_unquoted_variable_expansions_are_preserved_and_marked() {
    let info = parse_command_info("echo $? $$ $! $# $* $@ $1 $_NAME9", 0).unwrap();

    assert_eq!(
        info.args,
        vec!["$?", "$$", "$!", "$#", "$*", "$@", "$1", "$_NAME9"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    );
    assert!(info.has_variable_expansion);
    assert!(!info.has_glob);
}

#[test]
fn unquoted_glob_forms_are_marked() {
    let info = parse_command_info("echo *.rs file?.txt [ab].log", 0).unwrap();

    assert!(info.has_glob);
    assert!(!info.has_variable_expansion);
}

#[test]
fn parse_command_info_rejects_unclosed_quotes_backticks_and_escape() {
    let malformed =
        ["echo 'unterminated", "echo \"unterminated", "echo `unterminated", "echo trailing\\"];

    for command in malformed {
        assert!(parse_command_info(command, 0).is_none(), "command: {command}");
    }
}

#[test]
fn parse_redirect_recognizes_file_descriptor_duplicates() {
    let cases = [
        ("2>&1", RedirectKind::Stderr, "1"),
        ("1>&2", RedirectKind::Stdout, "2"),
        ("<&0", RedirectKind::Stdin, "0"),
    ];

    for (source, expected_kind, expected_target) in cases {
        let (redirect, consumed) = parse_redirect(&[token(source)], 0).unwrap();
        assert_eq!(redirect.kind, expected_kind, "source: {source}");
        assert_eq!(redirect.target, expected_target, "source: {source}");
        assert!(redirect.is_fd_duplicate, "source: {source}");
        assert_eq!(consumed, 1, "source: {source}");
    }
}

#[test]
fn command_parser_collects_file_descriptor_duplicates_as_redirects() {
    let info = parse_command_info("command 1>&2 2>&1", 0).unwrap();

    assert_eq!(info.name, "command");
    assert!(info.args.is_empty());
    assert_eq!(info.redirects.len(), 2);
    assert_eq!(
        info.redirects.iter().map(|redirect| redirect.kind).collect::<Vec<_>>(),
        vec![RedirectKind::Stdout, RedirectKind::Stderr]
    );
    assert_eq!(
        info.redirects.iter().map(|redirect| redirect.target.as_str()).collect::<Vec<_>>(),
        vec!["2", "1"]
    );
    assert!(info.redirects.iter().all(|redirect| redirect.is_fd_duplicate));
}

#[test]
fn parse_redirect_recognizes_inline_and_separate_file_targets() {
    let cases = [
        ("2>>", Some("errors.log"), RedirectKind::StderrAppend, "errors.log", 2),
        ("2>errors.log", None, RedirectKind::Stderr, "errors.log", 1),
        ("&>>all.log", None, RedirectKind::StderrAppend, "all.log", 1),
        ("&>all.log", None, RedirectKind::StdoutAndStderr, "all.log", 1),
        (">>", Some("output.log"), RedirectKind::Append, "output.log", 2),
        ("<<<payload", None, RedirectKind::Heredoc, "payload", 1),
        ("<<", Some("EOF"), RedirectKind::Heredoc, "EOF", 2),
        (">output.txt", None, RedirectKind::Stdout, "output.txt", 1),
        ("<", Some("input.txt"), RedirectKind::Stdin, "input.txt", 2),
    ];

    for (operator, separate_target, expected_kind, expected_target, expected_consumed) in cases {
        let mut tokens = vec![token(operator)];
        if let Some(target) = separate_target {
            tokens.push(token(target));
        }

        let (redirect, consumed) = parse_redirect(&tokens, 0).unwrap();
        assert_eq!(redirect.kind, expected_kind, "operator: {operator}");
        assert_eq!(redirect.target, expected_target, "operator: {operator}");
        assert!(!redirect.is_fd_duplicate, "operator: {operator}");
        assert_eq!(consumed, expected_consumed, "operator: {operator}");
    }
}

#[test]
fn parse_redirect_ignores_process_substitution_and_requires_target() {
    assert!(parse_redirect(&[token("<(sort input)")], 0).is_none());
    assert!(parse_redirect(&[token(">(cat)")], 0).is_none());
    assert!(parse_redirect(&[token(">")], 0).is_none());
    assert!(parse_redirect(&[], 0).is_none());
}

#[test]
fn top_level_splitters_ignore_operators_inside_nested_syntax() {
    let segments = split_top_level_pipeline(r#"echo "a|b" | cat $(printf '|') | diff <(echo "|")"#);
    assert_eq!(segments, vec![r#"echo "a|b""#, "cat $(printf '|')", r#"diff <(echo "|")"#]);

    let (left, operator, right) =
        split_top_level_compound(r#"echo "a && b" $(printf 'x;y') && next"#).unwrap();
    assert_eq!(left.trim(), r#"echo "a && b" $(printf 'x;y')"#);
    assert_eq!(operator, CompoundOp::And);
    assert_eq!(right.trim(), "next");
}

#[test]
fn subshell_inner_requires_one_balanced_outer_pair() {
    assert_eq!(subshell_inner("(echo (nested))"), Some("echo (nested)"));
    assert!(subshell_inner("echo (nested)").is_none());
    assert!(subshell_inner("(echo) trailing").is_none());
    assert!(subshell_inner("((echo)").is_none());
}

#[test]
fn parenthesized_reader_skips_quoted_nested_and_escaped_parentheses() {
    let input = r#"$(echo "(" $(printf ')') \(done\)) tail"#;
    let (inner, end) = read_command_substitution(input, 0).unwrap();

    assert_eq!(inner, r#"echo "(" $(printf ')') \(done\)"#);
    assert_eq!(&input[end..], " tail");
    assert!(read_command_substitution("$(unterminated", 0).is_none());
    assert!(read_process_substitution("<(unterminated", 0).is_none());
}

#[test]
fn backtick_reader_allows_escaped_delimiters() {
    let input = r#"`printf \`quoted\`` tail"#;
    let (inner, end) = read_backticks(input, 0).unwrap();

    assert_eq!(inner, r#"printf \`quoted\`"#);
    assert_eq!(&input[end..], " tail");
}

#[test]
fn variable_expansion_reader_accepts_supported_names_and_rejects_incomplete_forms() {
    let valid = ["$?", "$$", "$!", "$#", "$*", "$@", "$1", "$_NAME9", "$Name_2"];

    for input in valid {
        let (expansion, end) = read_variable_expansion(input, 0).unwrap();
        assert_eq!(expansion, input, "input: {input}");
        assert_eq!(end, input.len(), "input: {input}");
    }

    assert!(read_variable_expansion("$", 0).is_none());
    assert!(read_variable_expansion("${HOME", 0).is_none());
    assert!(read_variable_expansion("$-", 0).is_none());
    assert!(read_variable_expansion("plain", 0).is_none());
}

#[test]
fn braced_variable_expansion_is_preserved_and_marked() {
    let info = parse_command_info("printf ${HOME}/file", 0).unwrap();

    assert_eq!(info.args, vec!["${HOME}/file".to_string()]);
    assert!(info.has_variable_expansion);
}

#[test]
fn tokenizer_separates_redirect_attached_to_argument() {
    let info = parse_command_info("echo payload>output.txt", 0).unwrap();

    assert_eq!(info.name, "echo");
    assert_eq!(info.args, vec!["payload".to_string()]);
    assert_eq!(info.redirects.len(), 1);
    assert_eq!(info.redirects[0].kind, RedirectKind::Stdout);
    assert_eq!(info.redirects[0].target, "output.txt");
}

#[test]
fn redirect_reader_stops_before_top_level_operator() {
    let info = parse_command_info("echo ok>out.txt; next", 0).unwrap();

    assert_eq!(info.name, "echo");
    assert_eq!(info.redirects[0].target, "out.txt");
    assert_eq!(info.compound_operator, Some(CompoundOp::Sequence));
}

#[test]
fn variable_reader_handles_braces_and_nonzero_offsets() {
    let input = "prefix${USER}suffix";
    let start = "prefix".len();
    let (expansion, end) = read_variable_expansion(input, start).unwrap();

    assert_eq!(expansion, "${USER}");
    assert_eq!(&input[end..], "suffix");
}
