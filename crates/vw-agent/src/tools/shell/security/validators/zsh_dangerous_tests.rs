//! 验证 zsh 专属危险展开会被阻断而可移植命令保持允许。

use super::zsh_dangerous::ZshDangerousValidator;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityCategory, SecurityValidator};

#[test]
fn name_is_stable() {
    assert_eq!(ZshDangerousValidator.name(), "zsh_dangerous");
}

#[test]
fn blocks_zsh_only_expansions() {
    for command in ["=python --version", "echo ^old^new", "echo ~[1]"] {
        let findings = ZshDangerousValidator.validate(&parse_command(command));
        assert_eq!(findings[0].category, SecurityCategory::UnsafePattern);
    }
}

#[test]
fn allows_portable_shell_command() {
    assert!(ZshDangerousValidator.validate(&parse_command("python --version")).is_empty());
}
