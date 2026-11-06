//! Saved preferences cannot reauthorize permissions revoked by the host.
use super::*;

impl AcpClient {
    /// Resolve a restored permission value against the current host grant.
    /// Other controls (model, thinking, planning) retain their saved values.
    pub fn restored_permission_value<'a>(&self, option: &str, value: &'a str) -> &'a str {
        if self.full_access && self.permission_mode == PermissionMode::ApproveAll {
            return value;
        }
        self.restricted_permission_value(option, value)
    }

    /// Remove elevated values when capturing the baseline preceding full access.
    pub fn restricted_permission_value<'a>(&self, option: &str, value: &'a str) -> &'a str {
        use crate::access_policy::{
            claude, codebuddy, codex, goose, hermes, iflow, kimi, qoder, qwen,
        };
        if self.agent_name == "hermes" && option == "edit_approval_policy" && value != "ask" {
            return "ask";
        }
        if !matches!(option, "mode" | "session/set_mode") {
            return value;
        }
        match self.agent_name.as_str() {
            "codex" if value == codex::FULL_MODE => codex::RESTRICTED_MODE,
            "claude" if value == claude::FULL_MODE => claude::RESTRICTED_MODE,
            "qwen" if value == qwen::FULL_MODE => qwen::RESTRICTED_MODE,
            "hermes" if value != hermes::RESTRICTED_MODE => hermes::RESTRICTED_MODE,
            "iflow" if !iflow::restricted_mode(value) => iflow::RESTRICTED_MODE,
            "kimi" if !kimi::restricted_mode(value) => kimi::RESTRICTED_MODE,
            "codebuddy" if !codebuddy::restricted_mode(value) => codebuddy::RESTRICTED_MODE,
            "qoder" if !qoder::restricted_mode(value) => qoder::RESTRICTED_MODE,
            "goose" if !goose::restricted_mode(value) => goose::RESTRICTED_MODE,
            _ => value,
        }
    }
}
