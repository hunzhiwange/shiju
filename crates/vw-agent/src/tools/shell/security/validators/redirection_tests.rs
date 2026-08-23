//! 验证重定向校验器警告覆盖写入、阻断敏感路径并允许空设备输出。
//! 不同目标的严重级别必须保留差异，供上层策略决定告警或拒绝。

use super::redirection::RedirectionValidator;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityCategory, SecurityValidator, Severity};

#[test]
fn name_is_stable() {
    assert_eq!(RedirectionValidator.name(), "redirection");
}

#[test]
fn warns_for_overwrite_and_blocks_sensitive_paths() {
    let warning = RedirectionValidator.validate(&parse_command("echo ok > out.txt"));
    assert_eq!(warning[0].severity, Severity::Warn);

    let blocked = RedirectionValidator.validate(&parse_command("cat < /etc/passwd"));
    assert_eq!(blocked[0].category, SecurityCategory::DataExfiltration);
}

#[test]
fn allows_dev_null_stdout_redirection() {
    assert!(RedirectionValidator.validate(&parse_command("echo ok > /dev/null")).is_empty());
}
