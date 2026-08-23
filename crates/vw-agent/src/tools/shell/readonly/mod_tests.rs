//! 验证只读判定对回退解析、未知选项、变量展开与原地编辑的处理。

use crate::tools::shell::ast::{BashAst, CommandInfo, ParsedCommand, parse_command};
use crate::tools::shell::readonly::{ReadonlyCheckResult, check_readonly_constraints};

#[test]
fn readonly_result_helper_matches_only_readonly_variant() {
    assert!(ReadonlyCheckResult::Readonly.is_readonly());
    assert!(!ReadonlyCheckResult::UnknownFlag { flag: "--weird".into() }.is_readonly());
    assert!(!ReadonlyCheckResult::NotReadonly { reason: "nope".into() }.is_readonly());
}

#[test]
fn fallback_empty_command_is_not_readonly() {
    let cmd = ParsedCommand::Fallback { raw: String::new(), tokens: Vec::new() };
    assert_eq!(
        check_readonly_constraints(&cmd),
        ReadonlyCheckResult::NotReadonly { reason: "empty command".into() }
    );
}

#[test]
fn fallback_sed_without_in_place_stays_readonly() {
    let cmd = ParsedCommand::Fallback {
        raw: "sed -n 1p file.txt".into(),
        tokens: vec!["sed".into(), "-n".into(), "1p".into(), "file.txt".into()],
    };

    assert_eq!(check_readonly_constraints(&cmd), ReadonlyCheckResult::Readonly);
}

#[test]
fn sed_in_place_edit_is_not_readonly() {
    assert_eq!(
        check_readonly_constraints(&parse_command("sed -i 's/a/b/' file.txt")),
        ReadonlyCheckResult::NotReadonly { reason: "sed in-place edit is not read-only".into() }
    );
}

#[test]
fn readonly_check_rejects_unknown_git_subcommand() {
    let info = CommandInfo::from_command("git mystery").expect("command info");
    let ast = BashAst::parse("git mystery").0;
    let cmd = ParsedCommand::Ast(ast, info);

    assert_eq!(
        check_readonly_constraints(&cmd),
        ReadonlyCheckResult::NotReadonly { reason: "subcommand is not marked read-only".into() }
    );
}

#[test]
fn double_dash_stops_flag_validation() {
    assert_eq!(
        check_readonly_constraints(&parse_command("grep -- pattern --mystery")),
        ReadonlyCheckResult::Readonly
    );
}

#[test]
fn variable_expansion_marks_command_not_readonly() {
    assert_eq!(
        check_readonly_constraints(&parse_command("cat $FILE")),
        ReadonlyCheckResult::NotReadonly { reason: "unquoted glob or variable expansion".into() }
    );
}

#[test]
fn find_unquoted_glob_remains_readonly_without_variable_expansion() {
    assert_eq!(
        check_readonly_constraints(&parse_command("find . -name *.rs")),
        ReadonlyCheckResult::Readonly
    );
}
