//! 控制字符 validator。
//!
//! 阻断不可见控制字符，避免终端控制、日志欺骗或 shell 行为被隐藏字符改变。

use super::{SecurityCategory, SecurityValidator, block, has_control_characters, raw};
use crate::shell::ast::ParsedCommand;

pub(super) struct ControlCharsValidator;

impl SecurityValidator for ControlCharsValidator {
    fn name(&self) -> &str {
        "control_chars"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        if has_control_characters(raw(cmd)) {
            return vec![block(
                SecurityCategory::Obfuscation,
                "Command contains control characters that can alter terminal or shell behavior",
                Some("Remove control characters from the command"),
            )];
        }
        Vec::new()
    }
}
#[cfg(test)]
#[path = "control_chars_tests.rs"]
mod control_chars_tests;
