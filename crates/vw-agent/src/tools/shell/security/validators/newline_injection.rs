//! 换行注入 validator。
//!
//! Shell 工具一次应执行一条明确命令；引号外换行可能隐藏第二条命令，因此阻断。
//! 引号内换行是普通字符串内容，由可配置的 `quoted_newline` validator 单独处理。

use super::{SecurityCategory, SecurityValidator, block, has_unquoted_newline, raw};
use crate::tools::shell::ast::ParsedCommand;

pub(super) struct NewlineInjectionValidator;

impl SecurityValidator for NewlineInjectionValidator {
    fn name(&self) -> &str {
        "newline_injection"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::tools::shell::security::SecurityFinding> {
        let command = raw(cmd);
        if has_unquoted_newline(command) {
            return vec![block(
                SecurityCategory::Injection,
                "Unquoted newlines or carriage returns can inject additional shell commands",
                Some("Send one command per tool invocation"),
            )];
        }
        Vec::new()
    }
}
