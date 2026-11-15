//! mid-word hash 注释 validator。
//!
//! 阻断未引用且紧贴文本的 `#`，因为它容易让人工审阅以为后续内容仍会执行，或相反。

use super::{SecurityCategory, SecurityValidator, block, has_unquoted_hash, raw};
use crate::shell::ast::ParsedCommand;

pub(super) struct HashCommentValidator;

impl SecurityValidator for HashCommentValidator {
    fn name(&self) -> &str {
        "hash_comment"
    }

    fn validate(&self, cmd: &ParsedCommand) -> Vec<crate::shell::security::SecurityFinding> {
        if has_unquoted_hash(raw(cmd)) {
            return vec![block(
                SecurityCategory::Injection,
                "Mid-word hash comments can desynchronize what reviewers think will execute",
                Some("Move comments to their own line or remove them"),
            )];
        }
        Vec::new()
    }
}
#[cfg(test)]
#[path = "hash_comment_tests.rs"]
mod hash_comment_tests;
