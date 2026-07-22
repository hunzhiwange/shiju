//! Shell AST 解析测试，覆盖简单命令、包装器、重定向和解析质量等行为。

use super::{BashAst, CommandInfo, ParseQuality, ParsedCommand, parse_command, strip_wrappers};
use crate::app::agent::tools::shell::ast::{CompoundOp, RedirectKind};

#[test]
fn parse_simple_command() {
    let parsed = parse_command("ls -la /tmp");
    match parsed {
        ParsedCommand::Ast(_, info) => {
            assert_eq!(info.name, "ls");
            assert_eq!(info.args, vec!["-la", "/tmp"]);
        }
        ParsedCommand::Fallback { .. } => panic!("expected AST result"),
    }
}

#[test]
fn parse_quoted_argument() {
    let info =
        CommandInfo::from_command("echo 'hello world'").expect("quoted command should parse");
    assert_eq!(info.args, vec!["hello world"]);
}

#[test]
fn parse_pipeline_segments() {
    let info = CommandInfo::from_command("cat file | grep pattern | wc -l")
        .expect("pipeline should parse");
    assert_eq!(info.pipes.len(), 3);
    assert_eq!(info.pipes[1].info.name, "grep");
    assert_eq!(info.compound_operator, Some(CompoundOp::Pipe));
}

#[test]
fn parse_redirects() {
    let info = CommandInfo::from_command("echo hi > out.txt 2>&1").expect("redirects should parse");
    assert_eq!(info.redirects.len(), 2);
    assert_eq!(info.redirects[0].kind, RedirectKind::Stdout);
    assert_eq!(info.redirects[1].kind, RedirectKind::Stderr);
    assert!(info.redirects[1].is_fd_duplicate);
}

#[test]
fn parse_command_substitution() {
    let info =
        CommandInfo::from_command("echo $(date)").expect("command substitution should parse");
    assert!(info.has_command_substitution);
    assert_eq!(info.subcommands.len(), 1);
    assert_eq!(info.subcommands[0].name, "date");
}

#[test]
fn parse_process_substitution() {
    let info = CommandInfo::from_command("diff <(sort a) <(sort b)")
        .expect("process substitution should parse");
    assert!(info.has_process_substitution);
    assert_eq!(info.args, vec!["<(sort a)", "<(sort b)"]);
}

#[test]
fn parse_compound_command() {
    let info =
        CommandInfo::from_command("cd /tmp && ls -la").expect("compound command should parse");
    assert_eq!(info.name, "cd");
    assert_eq!(info.args, vec!["/tmp"]);
    assert_eq!(info.compound_operator, Some(CompoundOp::And));
}

#[test]
fn parse_brace_group_stays_on_ast_path() {
    let parsed = parse_command("{ echo hi; } > out.txt 2>&1");

    match parsed {
        ParsedCommand::Ast(_, info) => {
            assert_eq!(info.name, "echo");
            assert_eq!(info.redirects.len(), 2);
            assert_eq!(info.redirects[0].target, "out.txt");
        }
        ParsedCommand::Fallback { .. } => panic!("expected AST result"),
    }
}

#[test]
fn parse_failure_falls_back() {
    let (_, quality) = BashAst::parse("some {{ malformed");
    assert_eq!(quality, ParseQuality::Fallback);
}

#[test]
fn empty_string_falls_back() {
    let (_, quality) = BashAst::parse("   ");
    assert_eq!(quality, ParseQuality::Fallback);
}

#[test]
fn unsupported_delimiters_fall_back_and_preserve_original_source() {
    for command in [
        "echo 'unterminated",
        "echo \"unterminated",
        "echo `unterminated",
        "echo unexpected )",
        "echo unexpected }",
        "echo $(unterminated",
        "{ echo unterminated",
    ] {
        let (ast, quality) = BashAst::parse(command);

        assert_eq!(quality, ParseQuality::Fallback, "command: {command}");
        assert_eq!(ast.source(), command);
        assert!(ast.root_node().is_none());
    }
}

#[test]
fn repeated_parse_returns_equivalent_ast_and_quality() {
    let (first, first_quality) = BashAst::parse("printf cache-probe");
    let (second, second_quality) = BashAst::parse("printf cache-probe");

    assert_eq!(first, second);
    assert_eq!(first_quality, second_quality);
    assert_eq!(second.source(), "printf cache-probe");
}

#[test]
fn unicode_command_parses() {
    let info =
        CommandInfo::from_command("echo '你好，世界'").expect("unicode command should parse");
    assert_eq!(info.args, vec!["你好，世界"]);
}

#[test]
fn strip_timeout_and_nice_wrappers() {
    let info = CommandInfo::from_command("timeout 10 nice -n 19 ls")
        .expect("wrapper command should parse");
    let stripped = strip_wrappers(&info);
    assert_eq!(stripped.name, "ls");
    assert!(stripped.args.is_empty());
}

#[test]
fn strip_env_wrappers() {
    let info = CommandInfo::from_command("env FOO=bar env -u BAZ grep pattern")
        .expect("env wrapper should parse");
    let stripped = strip_wrappers(&info);
    assert_eq!(stripped.name, "grep");
    assert_eq!(stripped.args, vec!["pattern"]);
}
