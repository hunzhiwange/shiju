//! 验证进程环境文件访问会被阻断且错误信息不会泄露敏感载荷。
//! 其它 proc 路径不由本校验器泛化拒绝，以维持规则的单一职责。

use super::proc_environ::ProcEnvironValidator;
use crate::tools::shell::ast::parse_command;
use crate::tools::shell::security::{SecurityCategory, SecurityValidator};

#[test]
fn name_is_stable() {
    assert_eq!(ProcEnvironValidator.name(), "proc_environ");
}

#[test]
fn blocks_proc_environ_paths_without_echoing_secret_payloads() {
    let findings =
        ProcEnvironValidator.validate(&parse_command("cat /proc/self/environ SECRET=hidden"));

    assert_eq!(findings[0].category, SecurityCategory::DataExfiltration);
    assert!(!findings[0].message.contains("SECRET=hidden"));
}

#[test]
fn allows_other_proc_paths() {
    assert!(ProcEnvironValidator.validate(&parse_command("cat /proc/self/status")).is_empty());
}
