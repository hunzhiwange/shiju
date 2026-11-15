//! shell 元字符与动态执行 validator。
//!
//! 阻断危险 case 分隔符以及 exec/eval/source 等会改变执行边界的入口，避免静态校验
//! 被动态 shell 语义绕过。

use super::{SecurityCategory, SecurityValidator, block, raw};
use crate::shell::ast::ParsedCommand;

pub(super) struct MetacharactersValidator;

impl SecurityValidator for MetacharactersValidator {
    fn name(&self) -> &str {
        "metacharacters"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let command = raw(cmd);
        if command.contains(";;") || command.contains(";&") || command.contains(";;&") {
            return vec![block(
                SecurityCategory::Injection,
                "Command contains dangerous shell metacharacter chaining",
                Some("Split the command into explicit sequential invocations"),
            )];
        }
        let lowered = command.to_ascii_lowercase();
        if lowered.starts_with("exec ")
            || lowered.starts_with("eval ")
            || lowered.starts_with("source ")
            || lowered.starts_with(". ")
        {
            return vec![block(
                SecurityCategory::PrivilegeEscalation,
                "Command uses exec/eval/source, which can bypass static validation",
                Some("Invoke the concrete executable directly"),
            )];
        }
        Vec::new()
    }
}
