//! 验证 Unicode 空白校验器阻断非 ASCII 分隔符并允许普通空格。

use super::unicode_whitespace::UnicodeWhitespaceValidator;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityCategory, SecurityValidator};

#[test]
fn name_is_stable() {
    assert_eq!(UnicodeWhitespaceValidator.name(), "unicode_whitespace");
}

#[test]
fn blocks_non_ascii_whitespace() {
    let findings = UnicodeWhitespaceValidator.validate(&parse_command("echo\u{00a0}ok"));

    assert_eq!(findings[0].category, SecurityCategory::Obfuscation);
}

#[test]
fn allows_ascii_spaces() {
    assert!(UnicodeWhitespaceValidator.validate(&parse_command("echo ok")).is_empty());
}
