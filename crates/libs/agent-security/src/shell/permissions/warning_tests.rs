//! 破坏性命令警告测试，覆盖版本库、数据库和文件删除等高风险模式。

use crate::shell::ast::parse_command;

use super::warning::get_destructive_warning;

#[test]
fn destructive_warning_detects_git_reset_hard() {
    let warning = get_destructive_warning(&parse_command("git reset --hard HEAD~1"));
    assert_eq!(warning.as_deref(), Some("This will discard all uncommitted changes"));
}

#[test]
fn destructive_warning_detects_database_deletes() {
    let warning = get_destructive_warning(&parse_command("sqlite3 db.sqlite 'DELETE FROM users'"));
    assert_eq!(warning.as_deref(), Some("This will delete rows from a table"));
}

#[test]
fn destructive_warning_ignores_safe_commands() {
    assert!(get_destructive_warning(&parse_command("git status")).is_none());
}
