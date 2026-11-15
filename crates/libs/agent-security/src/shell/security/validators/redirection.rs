//! shell 重定向 validator。
//!
//! 检查文件描述符复制、敏感路径读写和覆盖式输出重定向。重定向会改变数据流向，是
//! shell 自动执行中最容易造成外泄或破坏的能力之一。

use super::{SecurityCategory, SecurityValidator, block, redirect_targets, warn};
use crate::shell::ast::{ParsedCommand, RedirectKind};

pub(super) struct RedirectionValidator;

impl SecurityValidator for RedirectionValidator {
    fn name(&self) -> &str {
        "redirection"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let mut findings = Vec::new();
        for (kind, target, is_fd_duplicate) in redirect_targets(cmd) {
            if is_fd_duplicate {
                findings.push(warn(
                    SecurityCategory::UnsafePattern,
                    "File descriptor duplication via redirection",
                    Some("Use direct stdout/stderr capture instead of exec-style fd manipulation"),
                ));
                continue;
            }
            if target.starts_with("/etc/")
                || target.starts_with("/proc/")
                || target.starts_with("/sys/")
                || target.starts_with("/root/")
            {
                findings.push(block(
                    SecurityCategory::DataExfiltration,
                    format!("Redirection target points to a sensitive path: {target}"),
                    Some("Redirect into a workspace-relative path instead"),
                ));
                continue;
            }
            if matches!(kind, RedirectKind::Stdout | RedirectKind::StdoutAndStderr)
                && target != "/dev/null"
            {
                // 普通覆盖重定向不总是恶意，但会修改文件；保留为警告让上层策略可见。
                findings.push(warn(
                    SecurityCategory::UnsafePattern,
                    format!("Output redirection overwrites an existing file target: {target}"),
                    Some("Use >> for append or capture output through the tool response"),
                ));
            }
        }
        findings
    }
}
