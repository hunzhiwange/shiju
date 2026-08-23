//! 验证混淆选项校验器阻断隐藏拼写并允许字面量选项。
//! 转义、空引号拼接等形式不得绕过后续只读与权限选项检查。

use super::obfuscated_flags::ObfuscatedFlagsValidator;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityCategory, SecurityValidator};

#[test]
fn name_is_stable() {
    assert_eq!(ObfuscatedFlagsValidator.name(), "obfuscated_flags");
}

#[test]
fn blocks_hidden_flag_spellings() {
    for command in [r"cmd $'\x2d\x2dhelp'", "cmd '' --force", "cmd 'x'-rf"] {
        let findings = ObfuscatedFlagsValidator.validate(&parse_command(command));
        assert_eq!(findings[0].category, SecurityCategory::Obfuscation);
    }
}

#[test]
fn allows_literal_flags() {
    assert!(ObfuscatedFlagsValidator.validate(&parse_command("cmd --help")).is_empty());
}
