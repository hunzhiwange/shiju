//! 验证 shell 元字符校验器阻断命令链与动态执行入口。
//! 单一普通命令不应产生发现项，避免元字符规则代替完整语法分类。

use super::metacharacters::MetacharactersValidator;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityCategory, SecurityValidator, Severity};

#[test]
fn name_is_stable() {
    assert_eq!(MetacharactersValidator.name(), "metacharacters");
}

#[test]
fn blocks_chaining_and_dynamic_shell_entrypoints() {
    for command in ["case x in a) echo ok;; esac", "eval echo secret", ". ./env"] {
        let findings = MetacharactersValidator.validate(&parse_command(command));
        assert_eq!(findings[0].severity, Severity::Block);
    }

    let findings = MetacharactersValidator.validate(&parse_command("exec ./tool"));
    assert_eq!(findings[0].category, SecurityCategory::PrivilegeEscalation);
    assert!(!findings[0].message.contains("secret-value"));
}

#[test]
fn allows_plain_command() {
    assert!(MetacharactersValidator.validate(&parse_command("echo ok")).is_empty());
}
