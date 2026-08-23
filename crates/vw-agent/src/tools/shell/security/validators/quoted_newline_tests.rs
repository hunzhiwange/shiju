//! 验证引号内换行校验器阻断跨行载荷并允许普通引号文本。
//! 该规则补足通用换行注入检查对引用区域有意保留的职责边界。

use super::quoted_newline::QuotedNewlineValidator;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityCategory, SecurityValidator};

#[test]
fn name_is_stable() {
    assert_eq!(QuotedNewlineValidator.name(), "quoted_newline");
}

#[test]
fn blocks_newline_inside_quotes() {
    for command in ["printf 'a\nb'", "printf \"a\rb\""] {
        let findings = QuotedNewlineValidator.validate(&parse_command(command));
        assert_eq!(findings[0].category, SecurityCategory::Injection);
    }
}

#[test]
fn allows_plain_quoted_text() {
    assert!(QuotedNewlineValidator.validate(&parse_command("printf 'ab'")).is_empty());
}
