//! 只读命令允许表测试，覆盖 git、文件查看和带危险标志命令的判定。

use crate::shell::ast::parse_command;
use crate::shell::readonly::{ReadonlyCheckResult, check_readonly_constraints};

#[test]
fn git_status_is_readonly() {
    assert_eq!(
        check_readonly_constraints(&parse_command("git status")),
        ReadonlyCheckResult::Readonly
    );
}

#[test]
fn git_commit_is_not_readonly() {
    assert!(matches!(
        check_readonly_constraints(&parse_command("git commit -m test")),
        ReadonlyCheckResult::NotReadonly { .. }
    ));
}

#[test]
fn grep_with_safe_flags_is_readonly() {
    assert_eq!(
        check_readonly_constraints(&parse_command("grep -ri pattern src")),
        ReadonlyCheckResult::Readonly
    );
}

#[test]
fn grep_with_pattern_file_is_not_readonly() {
    assert!(matches!(
        check_readonly_constraints(&parse_command("grep -f patterns word")),
        ReadonlyCheckResult::NotReadonly { .. }
    ));
}

#[test]
fn ls_is_readonly() {
    assert_eq!(check_readonly_constraints(&parse_command("ls -la")), ReadonlyCheckResult::Readonly);
}

#[test]
fn rm_is_not_readonly() {
    assert!(matches!(
        check_readonly_constraints(&parse_command("rm -rf target")),
        ReadonlyCheckResult::NotReadonly { .. }
    ));
}

#[test]
fn find_with_name_filter_is_readonly() {
    assert_eq!(
        check_readonly_constraints(&parse_command("find . -name '*.rs'")),
        ReadonlyCheckResult::Readonly
    );
}

#[test]
fn find_with_exec_is_not_readonly() {
    assert!(matches!(
        check_readonly_constraints(&parse_command("find . -exec rm {} \\;")),
        ReadonlyCheckResult::NotReadonly { .. }
    ));
}

#[test]
fn echo_is_readonly() {
    assert_eq!(
        check_readonly_constraints(&parse_command("echo hello")),
        ReadonlyCheckResult::Readonly
    );
}

#[test]
fn ps_is_readonly() {
    assert_eq!(check_readonly_constraints(&parse_command("ps aux")), ReadonlyCheckResult::Readonly);
}

#[test]
fn unknown_command_is_not_readonly() {
    assert!(matches!(
        check_readonly_constraints(&parse_command("unknown-command --foo")),
        ReadonlyCheckResult::NotReadonly { .. }
    ));
}

#[test]
fn unknown_flag_is_reported() {
    assert_eq!(
        check_readonly_constraints(&parse_command("grep --mystery pattern src")),
        ReadonlyCheckResult::UnknownFlag { flag: "--mystery".into() }
    );
}

#[test]
fn unquoted_glob_is_not_readonly() {
    assert!(matches!(
        check_readonly_constraints(&parse_command("ls *.rs")),
        ReadonlyCheckResult::NotReadonly { .. }
    ));
}
