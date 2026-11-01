//! Compiled event-aware Codex matcher. Invalid patterns never reach dispatch.
use super::hook_manifest::CodexHookEvent;
use anyhow::Result;

#[derive(Clone)]
pub(crate) enum HookMatcher {
    All,
    Exact(String),
    Regex(regex::Regex),
}
impl HookMatcher {
    pub(crate) fn new(event: CodexHookEvent, pattern: Option<&str>) -> Result<Self> {
        if matches!(
            event,
            CodexHookEvent::PreModelRequest
                | CodexHookEvent::PostModelRequest
                | CodexHookEvent::UserPromptSubmit
                | CodexHookEvent::Stop
                | CodexHookEvent::Interrupt
        ) {
            return Ok(Self::All);
        }
        let pattern = pattern.unwrap_or("");
        if pattern.is_empty() || pattern == "*" {
            return Ok(Self::All);
        }
        if pattern.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '|') {
            return Ok(Self::Exact(pattern.to_owned()));
        }
        Ok(Self::Regex(
            regex::Regex::new(pattern)
                .map_err(|_| anyhow::anyhow!("invalid Codex hook matcher"))?,
        ))
    }
    pub(crate) fn matches(&self, input: Option<&str>) -> bool {
        match self {
            Self::All => true,
            Self::Exact(pattern) => {
                input.is_some_and(|input| pattern.split('|').any(|candidate| candidate == input))
            }
            Self::Regex(regex) => input.is_some_and(|input| regex.is_match(input)),
        }
    }
}
