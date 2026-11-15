//! Shell 命令信息提取测试。
//!
//! 这些用例在启用 AST 特性时验证管道、命令替换和重定向能从语法树中直接提取。

use super::{BashAst, CommandInfo, CompoundOp, RedirectKind};

#[cfg(feature = "shell-ast")]
#[test]
fn from_ast_reads_pipeline_segments_directly() {
    let (ast, _) = BashAst::parse("cat file | grep needle | wc -l");
    let info = CommandInfo::from_ast(&ast).expect("pipeline should parse from ast");

    assert_eq!(info.name, "cat");
    assert_eq!(info.pipes.len(), 3);
    assert_eq!(info.pipes[1].info.name, "grep");
    assert_eq!(info.pipes[2].info.args, vec!["-l"]);
    assert_eq!(info.compound_operator, Some(CompoundOp::Pipe));
}

#[cfg(feature = "shell-ast")]
#[test]
fn from_ast_collects_subcommands_and_expansions() {
    let (ast, _) = BashAst::parse("echo $(printf '%s' \"$HOME\")");
    let info = CommandInfo::from_ast(&ast).expect("substitution should parse from ast");

    assert!(info.has_command_substitution);
    assert!(info.has_variable_expansion);
    assert_eq!(info.subcommands.len(), 1);
    assert_eq!(info.subcommands[0].name, "printf");
}

#[cfg(feature = "shell-ast")]
#[test]
fn from_ast_collects_redirects_on_redirected_statement() {
    let (ast, _) = BashAst::parse("{ echo hi; } > out.txt 2>&1");
    let info = CommandInfo::from_ast(&ast).expect("redirected statement should parse from ast");

    assert_eq!(info.name, "echo");
    assert_eq!(info.redirects.len(), 2);
    assert_eq!(info.redirects[0].kind, RedirectKind::Stdout);
    assert_eq!(info.redirects[0].target, "out.txt");
    assert_eq!(info.redirects[1].kind, RedirectKind::Stderr);
    assert!(info.redirects[1].is_fd_duplicate);
}
