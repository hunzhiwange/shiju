//! Native effort support: <https://platform.claude.com/docs/en/build-with-claude/effort>
use crate::core::language_model::ReasoningEffort;

pub(super) fn validate_effort(model: &str, effort: Option<ReasoningEffort>) -> crate::Result<()> {
    let Some(effort) = effort else {
        return Ok(());
    };
    let matches = |id: &str| {
        model == id
            || model
                .strip_prefix(id)
                .and_then(|s| s.strip_prefix('-'))
                .is_some_and(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '-'))
    };
    let extended = [
        "claude-opus-4-7",
        "claude-opus-4-8",
        "claude-opus-5",
        "claude-opus-5-5",
        "claude-sonnet-5",
        "claude-sonnet-5-5",
        "claude-haiku-5-5",
        "claude-fable-5",
        "claude-fable-5-1",
        "claude-mythos-5",
        "claude-mythos-5-1",
    ]
    .iter()
    .any(|id| matches(id));
    let max = extended
        || [
            "claude-opus-4-6",
            "claude-sonnet-4-6",
            "claude-mythos-preview",
        ]
        .iter()
        .any(|id| matches(id));
    let standard = max || matches("claude-opus-4-5");
    let supported = match effort {
        ReasoningEffort::None => {
            matches("claude-opus-4-5") || matches("claude-opus-4-6") || matches("claude-sonnet-4-6")
        }
        ReasoningEffort::Minimal => false,
        ReasoningEffort::Low | ReasoningEffort::Medium | ReasoningEffort::High => standard,
        ReasoningEffort::Xhigh => extended,
        ReasoningEffort::Max => max,
    };
    if supported {
        Ok(())
    } else {
        Err(crate::Error::InvalidInput(format!(
            "model {model} does not support reasoning_effort={}",
            effort.as_str()
        )))
    }
}

#[cfg(test)]
#[path = "reasoning_tests.rs"]
mod tests;
