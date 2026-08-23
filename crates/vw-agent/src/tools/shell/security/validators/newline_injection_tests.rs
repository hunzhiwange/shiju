//! 验证换行注入校验器区分命令分隔换行与引号内文本。
//! LF 与 CR 都属于命令边界风险，但合法引用内容由专门规则处理。

use super::newline_injection::NewlineInjectionValidator;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityCategory, SecurityValidator};

#[test]
fn name_is_stable() {
    assert_eq!(NewlineInjectionValidator.name(), "newline_injection");
}

#[test]
fn blocks_lf_and_cr() {
    for command in ["echo one\necho two", "echo one\recho two"] {
        let findings = NewlineInjectionValidator.validate(&parse_command(command));
        assert_eq!(findings[0].category, SecurityCategory::Injection);
    }
}

#[test]
fn allows_single_line_command() {
    assert!(NewlineInjectionValidator.validate(&parse_command("echo one")).is_empty());
}

#[test]
fn allows_quoted_line_breaks() {
    for command in ["printf 'one\ntwo'", "printf \"one\rtwo\""] {
        assert!(NewlineInjectionValidator.validate(&parse_command(command)).is_empty());
    }
}
