//! Opaque selection of an already registered account on the authorized execution host.
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClaudeLoginBinding {
    pub account_id: String,
}
impl std::fmt::Debug for ClaudeLoginBinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ClaudeLoginBinding([redacted])")
    }
}
