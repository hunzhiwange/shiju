//! Shell AST 数据结构测试。
//!
//! 这些测试确保命令信息结构可序列化往返，并锁定枚举变体与嵌套命令替换的基础行为。

use super::{CommandInfo, CompoundOp, Redirect, RedirectKind};

#[test]
fn command_info_round_trip_serialization() {
    let info = CommandInfo {
        name: "grep".to_string(),
        args: vec!["pattern".to_string(), "file.txt".to_string()],
        redirects: vec![Redirect {
            kind: RedirectKind::Stdout,
            target: "out.txt".to_string(),
            is_fd_duplicate: false,
        }],
        pipes: Vec::new(),
        subcommands: vec![CommandInfo {
            name: "date".to_string(),
            args: vec!["+%F".to_string()],
            redirects: Vec::new(),
            pipes: Vec::new(),
            subcommands: Vec::new(),
            has_command_substitution: false,
            has_process_substitution: false,
            has_glob: false,
            has_variable_expansion: false,
            compound_operator: None,
        }],
        has_command_substitution: true,
        has_process_substitution: false,
        has_glob: false,
        has_variable_expansion: false,
        compound_operator: Some(CompoundOp::Pipe),
    };

    let json = serde_json::to_string(&info).expect("command info should serialize");
    let decoded: CommandInfo =
        serde_json::from_str(&json).expect("command info should deserialize");
    assert_eq!(decoded, info);
}

#[test]
fn redirect_kind_variants_are_constructible() {
    let kinds = [
        RedirectKind::Stdin,
        RedirectKind::Stdout,
        RedirectKind::Stderr,
        RedirectKind::Append,
        RedirectKind::StdoutAndStderr,
        RedirectKind::StderrAppend,
        RedirectKind::Heredoc,
    ];
    assert_eq!(kinds.len(), 7);
}

#[test]
fn compound_op_variants_are_constructible() {
    let ops = [
        CompoundOp::And,
        CompoundOp::Or,
        CompoundOp::Sequence,
        CompoundOp::Pipe,
        CompoundOp::Subshell,
    ];
    assert_eq!(ops.len(), 5);
}

#[test]
fn nested_command_substitution_depth_is_retained() {
    let info = CommandInfo::from_command("echo $(printf $(date +%F))")
        .expect("nested substitutions should parse");
    assert_eq!(info.subcommands.len(), 1);
    assert_eq!(info.subcommands[0].name, "printf");
    assert_eq!(info.subcommands[0].subcommands.len(), 1);
    assert_eq!(info.subcommands[0].subcommands[0].name, "date");
}
