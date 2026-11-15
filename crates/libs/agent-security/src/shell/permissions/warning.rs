//! 破坏性命令警告逻辑，负责为高风险 Shell 操作生成用户可读提醒。

use regex::Regex;

use crate::shell::ast::ParsedCommand;

use super::RulePattern;

/// 执行 get_destructive_warning 操作，并返回调用方需要的结果。
pub fn get_destructive_warning(cmd: &ParsedCommand) -> Option<String> {
    let raw = cmd.raw().trim();
    let patterns = [
        (
            RulePattern::Prefix { command: "git reset --hard".into() },
            "This will discard all uncommitted changes",
        ),
        (
            RulePattern::Prefix { command: "git push --force".into() },
            "This will overwrite remote history",
        ),
        (
            RulePattern::Prefix { command: "git clean -f".into() },
            "This will permanently delete untracked files",
        ),
        (
            RulePattern::Prefix { command: "git checkout .".into() },
            "This will discard all working directory changes",
        ),
        (
            RulePattern::Prefix { command: "git restore .".into() },
            "This will discard all working directory changes",
        ),
        (
            RulePattern::Prefix { command: "git stash drop".into() },
            "This will permanently delete a stash",
        ),
        (
            RulePattern::Prefix { command: "git stash clear".into() },
            "This will permanently delete all stashes",
        ),
        (RulePattern::Glob { pattern: "rm -rf *".into() }, "This will recursively delete files"),
        (RulePattern::Glob { pattern: "rm -r *".into() }, "This will recursively delete files"),
        (
            RulePattern::Regex {
                pattern: Regex::new(r"(?i)\bDROP\s+TABLE\b").expect("valid regex"),
            },
            "This will permanently delete a database table",
        ),
        (
            RulePattern::Regex {
                pattern: Regex::new(r"(?i)\bTRUNCATE\s+TABLE\b").expect("valid regex"),
            },
            "This will delete all rows in a table",
        ),
        (
            RulePattern::Regex {
                pattern: Regex::new(r"(?i)\bDELETE\s+FROM\b").expect("valid regex"),
            },
            "This will delete rows from a table",
        ),
        (
            RulePattern::Prefix { command: "kubectl delete".into() },
            "This will delete a Kubernetes resource",
        ),
        (
            RulePattern::Prefix { command: "terraform destroy".into() },
            "This will destroy all Terraform-managed infrastructure",
        ),
    ];

    patterns
        .iter()
        .find_map(|(pattern, warning)| pattern.matches_raw(raw).then(|| (*warning).to_string()))
}
