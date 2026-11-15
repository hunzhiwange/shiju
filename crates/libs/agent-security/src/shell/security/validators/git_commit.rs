//! git commit 消息注入 validator。
//!
//! 专门检查 `git commit -m` 中的命令替换/进程替换，避免提交消息参数在 shell 层触发
//! 额外命令执行。

use super::{SecurityCategory, SecurityValidator, block, info, raw};
use crate::shell::ast::ParsedCommand;

pub(super) struct GitCommitValidator;

impl SecurityValidator for GitCommitValidator {
    fn name(&self) -> &str {
        "git_commit"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        let Some(info) = info(cmd) else {
            return Vec::new();
        };
        if info.name != "git" || !info.args.iter().any(|arg| arg == "commit") {
            return Vec::new();
        }
        let command = raw(cmd);
        if command.contains(" -m \"$(")
            || command.contains(" -m '$(")
            || command.contains(" -m `")
            || command.contains(" -m \"`")
            || command.contains(" -m '<(")
        {
            return vec![block(
                SecurityCategory::Injection,
                "git commit message contains command substitution or process substitution",
                Some("Pass a literal commit message to -m"),
            )];
        }
        Vec::new()
    }
}
#[cfg(test)]
#[path = "git_commit_tests.rs"]
mod git_commit_tests;
