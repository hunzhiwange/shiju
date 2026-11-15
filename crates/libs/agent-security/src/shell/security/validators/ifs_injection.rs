//! IFS 注入 validator。
//!
//! IFS 会改变 shell 对参数的分词规则；攻击者可以借此绕过静态 token 检查，因此出现
//! 赋值或引用时直接阻断。

use super::{SecurityCategory, SecurityValidator, block, raw};
use crate::shell::ast::ParsedCommand;

pub(super) struct IfsInjectionValidator;

impl SecurityValidator for IfsInjectionValidator {
    fn name(&self) -> &str {
        "ifs_injection"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let command = raw(cmd);
        if command.contains("IFS=") || command.contains("${IFS}") || command.contains("$IFS") {
            return vec![block(
                SecurityCategory::Injection,
                "IFS manipulation changes shell tokenization and is blocked",
                Some("Pass literal whitespace-separated arguments instead"),
            )];
        }
        Vec::new()
    }
}
#[cfg(test)]
#[path = "ifs_injection_tests.rs"]
mod ifs_injection_tests;
