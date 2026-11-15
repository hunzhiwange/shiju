//! Unicode 空白混淆 validator。
//!
//! 阻断非 ASCII 空白字符，避免看起来像普通空格但不会被相同规则处理的字符绕过检查。

use super::{SecurityCategory, SecurityValidator, block, contains_unicode_whitespace, raw};
use crate::shell::ast::ParsedCommand;

pub(super) struct UnicodeWhitespaceValidator;

impl SecurityValidator for UnicodeWhitespaceValidator {
    fn name(&self) -> &str {
        "unicode_whitespace"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        if contains_unicode_whitespace(raw(cmd)) {
            return vec![block(
                SecurityCategory::Obfuscation,
                "Command contains Unicode whitespace that may bypass shell token checks",
                Some("Replace the whitespace with plain ASCII spaces"),
            )];
        }
        Vec::new()
    }
}
