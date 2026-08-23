//! 验证只读命令长短选项的精确匹配与聚合形式判定。
//! 带值长选项可继承安全性，短选项组合则不得绕过显式不安全标志。

use super::*;
use crate::tools::shell::readonly::CommandAllowlistEntry;

fn entry() -> CommandAllowlistEntry {
    CommandAllowlistEntry {
        command: "demo",
        subcommands: None,
        safe_flags: &["--color", "-a"],
        unsafe_flags: &["--write", "-x"],
        allow_any_flag: false,
    }
}

#[test]
fn long_flags_match_exactly_or_with_value() {
    let entry = entry();
    assert!(is_safe_flag("--color", &entry));
    assert!(is_safe_flag("--color=always", &entry));
    assert!(!is_safe_flag("--colorful", &entry));
}

#[test]
fn short_flags_allow_aggregated_suffixes() {
    let entry = entry();
    assert!(is_safe_flag("-abc", &entry));
    assert!(is_unsafe_flag("-xz", &entry));
}
