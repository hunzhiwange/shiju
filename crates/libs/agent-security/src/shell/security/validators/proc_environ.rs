//! `/proc/*/environ` 访问 validator。
//!
//! 读取进程环境可能暴露 token、密钥和服务凭据，因此该路径在 shell 命令中按数据外泄
//! 风险阻断。

use super::{SecurityCategory, SecurityValidator, block, raw};
use crate::shell::ast::ParsedCommand;
use regex::Regex;
use std::sync::LazyLock;

static PROC_ENVIRON_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/proc/(\*|self|\d+)/environ").expect("valid proc environ regex"));

pub(super) struct ProcEnvironValidator;

impl SecurityValidator for ProcEnvironValidator {
    fn name(&self) -> &str {
        "proc_environ"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        if PROC_ENVIRON_RE.is_match(raw(cmd)) {
            return vec![block(
                SecurityCategory::DataExfiltration,
                "Reading /proc/*/environ can expose secrets from another process",
                Some("Use explicit, non-sensitive environment inputs instead"),
            )];
        }
        Vec::new()
    }
}
